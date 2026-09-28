use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Root, Sizable as _, TitleBar, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::*;

use crate::config::{self, Config, WindowState};
use crate::home::HomeView;
use crate::jellyfin::{Api, Item, Kind, Session};
use crate::library::{LibraryView, SortChanged};
use crate::login::{LoggedIn, LoginView};
use crate::movie::MovieView;
use crate::nav::Nav;
use crate::player::{Closed, PlayerView};
use crate::search::{SearchEvent, SearchView};
use crate::series::SeriesView;
use crate::settings::{SettingsChanged, SettingsView};
use crate::shortcuts::{self, ShowShortcuts};
use crate::update;

actions!(tbis, [FocusSearch]);

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Home,
    Movies,
    Series,
}

const TABS: [(Tab, &str); 3] = [
    (Tab::Home, "Home"),
    (Tab::Movies, "Movies"),
    (Tab::Series, "Series"),
];

struct Main {
    session: Session,
    api: Api,
    tab: Tab,
    home: Entity<HomeView>,
    movies: Entity<LibraryView>,
    series: Entity<LibraryView>,
    search_input: Entity<InputState>,
    search: Entity<SearchView>,
    /// Search input expanded in title bar; collapses to an icon on blur.
    search_open: bool,
    /// Detail pages over the current tab, innermost last; Back pops one
    /// (e.g. Episode back to its Series).
    details: Vec<Detail>,
    player: Option<(Entity<PlayerView>, Subscription)>,
    _subscriptions: [Subscription; 8],
    /// Waits for `Api`'s 401 signal; dropped with Main so a stale one can't fire later.
    _expired: Task<()>,
}

/// Detail page shown over the current tab; opened from a Movie or Series poster.
struct Detail {
    view: AnyView,
    refresh: Box<dyn Fn(&mut App)>,
    // kept alive so Nav (and e.g. Settings changes) from the detail view keep arriving
    _subscriptions: Vec<Subscription>,
}

impl Detail {
    fn refresh(&self, cx: &mut App) {
        (self.refresh)(cx)
    }
}

impl AppView {
    /// Saves volume/track prefs and snapshots a final playback report, if any Player is
    /// open. Called from both quit paths (see the two `on_app_quit`/`on_window_should_close`
    /// registrations in `new`) since either one may run while the other silently no-ops.
    fn prepare_quit(&mut self, cx: &mut Context<Self>) -> Option<impl Future<Output = ()> + use<>> {
        let mut report = None;
        if let Screen::Main(main) = &self.screen
            && let Some((player, _)) = &main.player
        {
            let (volume, muted) = player.read(cx).volume_state();
            self.config.volume = volume;
            self.config.muted = muted;
            self.config.track_prefs = player.read(cx).track_prefs();
            report = player.read(cx).quit_report();
        }
        save_config(&self.config, "volume/window");
        report
    }
}

impl Main {
    fn searching(&self, cx: &App) -> bool {
        !self.search_input.read(cx).value().trim().is_empty()
    }

    /// Reloads whatever is on screen so played state and progress are fresh.
    fn refresh_visible(&self, cx: &mut App) {
        if let Some(detail) = self.details.last() {
            detail.refresh(cx);
        } else {
            match self.tab {
                Tab::Movies => self.movies.update(cx, |view, cx| view.refresh(cx)),
                Tab::Series => self.series.update(cx, |view, cx| view.refresh(cx)),
                Tab::Home => self.home.update(cx, |view, cx| view.refresh(cx)),
            }
        }
    }
}

#[allow(clippy::large_enum_variant)] // one instance
enum Screen {
    Login {
        view: Entity<LoginView>,
        _subscription: Subscription,
    },
    Main(Main),
}

pub struct AppView {
    config: Config,
    screen: Screen,
    /// Keeps app-wide shortcuts (⌘F) reachable when nothing else has focus.
    focus: FocusHandle,
    /// Tracks size/position into `config.window`; saved with the rest on quit.
    _window_bounds: Subscription,
    /// Update check or install in flight; replacing it cancels the old one.
    _update: Task<()>,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = config::load();
        cx.set_global(config.seek);
        cx.set_global(config.hide_spoilers);
        let screen = match config.session.clone() {
            Some(session) => Self::main_screen(session, &config, window, cx),
            None => Self::login_screen(&config, window, cx),
        };
        let focus = cx.focus_handle();
        if matches!(screen, Screen::Main(_)) {
            focus.focus(window, cx);
        }
        // cmd-Q: entity's still alive when this runs (shutdown() awaits quit_observers
        // before dropping windows), so the weak-entity update below succeeds directly.
        cx.on_app_quit(|this, cx| {
            let report = this.prepare_quit(cx);
            async move {
                if let Some(report) = report {
                    report.await;
                }
            }
        })
        .detach();
        // Window close (red button): the window (and this view) is torn down *before*
        // `on_app_quit` runs, so the callback above silently no-ops there. Snapshot the
        // report now, while still alive, and hand it to a plain, entity-free `on_app_quit`
        // so it still gets the shutdown budget regardless of teardown order.
        let weak = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_window, cx| {
            if let Some(report) = weak
                .update(cx, |this, cx| this.prepare_quit(cx))
                .ok()
                .flatten()
            {
                let mut report = Some(report);
                cx.on_app_quit(move |_cx| {
                    let report = report.take();
                    async move {
                        if let Some(report) = report {
                            report.await;
                        }
                    }
                })
                .detach();
            }
            true
        });
        let _window_bounds = cx.observe_window_bounds(window, |this, window, _| {
            this.config.window = Some(WindowState::from(window.window_bounds()));
        });
        let mut this = Self {
            config,
            screen,
            focus,
            _window_bounds,
            _update: Task::ready(()),
        };
        this.check_for_updates(false, window, cx);
        this
    }

    /// Looks for a newer release; `manual` (menu) also reports "up to date" and errors.
    pub fn check_for_updates(&mut self, manual: bool, window: &mut Window, cx: &mut Context<Self>) {
        if update::bundle_path().is_none() {
            if manual {
                let note = Notification::info("Updates only work in the installed tbis.app.");
                window.push_notification(note, cx);
            }
            return;
        }
        let http = cx.http_client();
        self._update = cx.spawn_in(window, async move |this, cx| {
            let result = update::check(http).await;
            this.update_in(cx, |this, window, cx| match result {
                Ok(Some(update)) => this.offer_update(update, window, cx),
                Ok(None) if manual => {
                    let note = format!("tbis {} is the latest version.", update::CURRENT);
                    window.push_notification(Notification::success(note), cx);
                }
                Ok(None) => {}
                Err(err) if manual => {
                    let note = Notification::error(format!("Update check failed: {err}"));
                    window.push_notification(note, cx);
                }
                // automatic check: offline or rate-limited is no reason to interrupt
                Err(err) => eprintln!("update check failed: {err}"),
            })
            .ok();
        });
    }

    fn offer_update(
        &mut self,
        update: update::Update,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let this = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (this, update) = (this.clone(), update.clone());
            alert
                .title(format!("tbis {} is available", update.version))
                .description(format!(
                    "You have {}. tbis restarts to finish updating.",
                    update::CURRENT
                ))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Install and Restart")
                        .cancel_text("Later"),
                )
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    let update = update.clone();
                    this.update(cx, |this, cx| this.install_update(update, window, cx))
                        .ok();
                    true
                })
        });
    }

    fn install_update(
        &mut self,
        update: update::Update,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let note = Notification::info(format!("Downloading tbis {}\u{2026}", update.version));
        window.push_notification(note, cx);
        let http = cx.http_client();
        self._update = cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(async move { update::install(http, &update).await })
                .await
                .and_then(|()| update::relaunch());
            this.update_in(cx, |_, window, cx| match result {
                Ok(()) => cx.quit(),
                Err(err) => {
                    let note = Notification::error(format!("Update failed: {err}"));
                    window.push_notification(note, cx);
                }
            })
            .ok();
        });
    }

    fn login_screen(config: &Config, window: &mut Window, cx: &mut Context<Self>) -> Screen {
        let login = cx.new(|cx| LoginView::new(config.device_id.clone(), window, cx));
        let subscription =
            cx.subscribe_in(&login, window, |this, _, LoggedIn(session), window, cx| {
                this.config.session = Some(session.clone());
                save_config(&this.config, "session");
                this.screen = Self::main_screen(session.clone(), &this.config, window, cx);
                this.focus.focus(window, cx);
                cx.notify();
            });
        Screen::Login {
            view: login,
            _subscription: subscription,
        }
    }

    fn main_screen(
        session: Session,
        config: &Config,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Screen {
        let (expired_tx, mut expired_rx) = futures::channel::mpsc::unbounded();
        let api = Api::new(
            cx.http_client(),
            session.clone(),
            config.device_id.clone(),
            expired_tx,
        );
        let _expired = cx.spawn_in(window, async move |this, cx| {
            if futures::StreamExt::next(&mut expired_rx).await.is_some() {
                this.update_in(cx, |this, window, cx| this.session_expired(window, cx))
                    .ok();
            }
        });
        let home = cx.new(|cx| {
            let mut home = HomeView::new(api.clone());
            home.refresh(cx);
            home
        });
        let movies = cx.new(|_| LibraryView::new(Kind::Movie, api.clone(), config.movies_sort));
        let series = cx.new(|_| LibraryView::new(Kind::Series, api.clone(), config.series_sort));
        let local = [movies.clone(), series.clone()];
        let recent = config.recent_searches.clone();
        let search = cx.new(|_| SearchView::new(api.clone(), local, recent));
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search")
                .clean_on_escape()
        });
        let _subscriptions = [
            cx.subscribe(&movies, |this, _, SortChanged(sort), _| {
                this.config.movies_sort = *sort;
                save_config(&this.config, "sort");
            }),
            cx.subscribe(&series, |this, _, SortChanged(sort), _| {
                this.config.series_sort = *sort;
                save_config(&this.config, "sort");
            }),
            cx.subscribe_in(&home, window, |this, _, nav, window, cx| {
                this.on_nav(nav, window, cx)
            }),
            cx.subscribe_in(&movies, window, |this, _, nav, window, cx| {
                this.on_nav(nav, window, cx)
            }),
            cx.subscribe_in(&series, window, |this, _, nav, window, cx| {
                this.on_nav(nav, window, cx)
            }),
            cx.subscribe_in(&search, window, |this, search, nav, window, cx| {
                // opening a result is what makes a query worth keeping
                search.update(cx, |search, cx| search.remember(cx));
                this.on_nav(nav, window, cx)
            }),
            cx.subscribe_in(
                &search,
                window,
                |this, search, event: &SearchEvent, window, cx| {
                    match event {
                        SearchEvent::Pick(query) => {
                            let Screen::Main(main) = &mut this.screen else {
                                return;
                            };
                            // set_value emits no Change, so run the query directly
                            main.search_input.update(cx, |input, cx| {
                                input.set_value(query.clone(), window, cx);
                                input.focus(window, cx);
                            });
                            main.search_open = true;
                            search.update(cx, |search, cx| search.set_query(query, cx));
                            cx.notify();
                        }
                        SearchEvent::RecentChanged(recent) => {
                            this.config.recent_searches = recent.clone();
                            save_config(&this.config, "recent searches");
                            cx.notify();
                        }
                    }
                },
            ),
            cx.subscribe(&search_input, |this, input, event: &InputEvent, cx| {
                let Screen::Main(main) = &mut this.screen else {
                    return;
                };
                match event {
                    InputEvent::Change => {
                        let query = input.read(cx).value();
                        main.search
                            .update(cx, |search, cx| search.set_query(&query, cx));
                        cx.notify();
                    }
                    InputEvent::Blur => {
                        main.search_open = false;
                        cx.notify();
                    }
                    InputEvent::PressEnter { .. } => {
                        main.search.update(cx, |search, cx| search.remember(cx));
                    }
                    _ => {}
                }
            }),
        ];
        Screen::Main(Main {
            session,
            api,
            tab: Tab::Home,
            home,
            movies,
            series,
            search_input,
            search,
            search_open: false,
            details: Vec::new(),
            player: None,
            _subscriptions,
            _expired,
        })
    }

    fn select_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &mut self.screen else {
            return;
        };
        main.tab = TABS[index].0;
        main.details.clear();
        Self::clear_search(main, window, cx);
        main.refresh_visible(cx);
        cx.notify();
    }

    fn clear_search(main: &Main, window: &mut Window, cx: &mut App) {
        main.search_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        main.search
            .update(cx, |search, cx| search.set_query("", cx));
    }

    /// Every Nav-emitting view (Home/Library/Search rows, Movie/Series detail) routes here.
    fn on_nav(&mut self, nav: &Nav, window: &mut Window, cx: &mut Context<Self>) {
        match nav {
            Nav::Open(item) => match item.kind {
                Kind::Series => self.open_series(item, window, cx),
                // Episode detail shares the Movie page; Other would be a server data bug
                Kind::Movie | Kind::Episode | Kind::Other => self.open_movie(item, window, cx),
            },
            Nav::Play(item) => self.open_player(item, window, cx),
            Nav::Back => {
                let Screen::Main(main) = &mut self.screen else {
                    return;
                };
                main.details.pop();
                main.refresh_visible(cx);
                cx.notify();
            }
        }
    }

    /// Opens `view` as the detail page: subscribes it to Nav, remembers how to refresh it.
    fn set_detail<V: Render + EventEmitter<Nav>>(
        &mut self,
        view: Entity<V>,
        refresh: impl Fn(&mut App) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let _nav = cx.subscribe_in(&view, window, |this, _, nav, window, cx| {
            this.on_nav(nav, window, cx)
        });
        let Screen::Main(main) = &mut self.screen else {
            return;
        };
        main.details.push(Detail {
            view: view.into(),
            refresh: Box::new(refresh),
            _subscriptions: vec![_nav],
        });
        Self::clear_search(main, window, cx);
        cx.notify();
    }

    fn open_series(&mut self, series: &Item, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &self.screen else {
            return;
        };
        let (api, series) = (main.api.clone(), series.clone());
        let detail = cx.new(|cx| SeriesView::new(api, series, cx));
        let refresh_view = detail.clone();
        self.set_detail(
            detail,
            move |cx| refresh_view.update(cx, |view, cx| view.refresh(cx)),
            window,
            cx,
        );
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &self.screen else {
            return;
        };
        let (config, api) = (&self.config, main.api.clone());
        let view = cx.new(|cx| SettingsView::new(config, api, cx));
        let changed = cx.subscribe(&view, |this, _, changed: &SettingsChanged, cx| {
            match changed {
                SettingsChanged::Language(language) => this.config.language = language.clone(),
                SettingsChanged::Subtitles(style) => this.config.subtitles = style.clone(),
                SettingsChanged::MaxBitrate(cap) => this.config.max_bitrate_mbps = *cap,
                SettingsChanged::Shaders(profile) => this.config.shaders = *profile,
                SettingsChanged::Seek(steps) => {
                    this.config.seek = *steps;
                    cx.set_global(*steps);
                }
                SettingsChanged::Theme(name) => this.config.theme = Some(name.clone()),
                SettingsChanged::HideSpoilers(hide) => {
                    this.config.hide_spoilers = *hide;
                    cx.set_global(*hide);
                }
            }
            save_config(&this.config, "settings");
        });
        self.set_detail(view, |_| {}, window, cx);
        if let Screen::Main(main) = &mut self.screen
            && let Some(detail) = main.details.last_mut()
        {
            detail._subscriptions.push(changed);
        }
    }

    fn open_movie(&mut self, movie: &Item, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &self.screen else {
            return;
        };
        let (api, movie) = (main.api.clone(), movie.clone());
        let detail = cx.new(|cx| MovieView::new(api, movie, cx));
        let refresh_view = detail.clone();
        self.set_detail(
            detail,
            move |cx| refresh_view.update(cx, |view, cx| view.refresh(cx)),
            window,
            cx,
        );
    }

    fn focus_search(&mut self, _: &FocusSearch, window: &mut Window, cx: &mut Context<Self>) {
        if let Screen::Main(main) = &mut self.screen
            && main.player.is_none()
        {
            main.search_open = true;
            cx.notify();
            main.search_input
                .update(cx, |input, cx| input.focus(window, cx));
        }
    }

    fn open_player(&mut self, item: &Item, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &mut self.screen else {
            return;
        };
        let api = main.api.clone();
        let config = &self.config;
        let player = cx.new(|cx| PlayerView::new(api, item, config, window, cx));
        let subscription =
            cx.subscribe_in(&player, window, |this, _, closed: &Closed, window, cx| {
                this.config.volume = closed.volume;
                this.config.muted = closed.muted;
                this.config.track_prefs = closed.track_prefs.clone();
                save_config(&this.config, "volume");
                this.close_player(window, cx)
            });
        main.player = Some((player, subscription));
        set_video_background(true, window, cx);
        cx.notify();
    }

    fn close_player(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &mut self.screen else {
            return;
        };
        main.player = None; // drops mpv
        set_video_background(false, window, cx);
        main.refresh_visible(cx);
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// Server rejected the token: leave the Player (keeping its volume/tracks), then
    /// sign in again with the server prefilled.
    fn session_expired(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &self.screen else {
            return;
        };
        let server = main.session.server.clone();
        if let Some((player, _)) = &main.player {
            let player = player.read(cx);
            (self.config.volume, self.config.muted) = player.volume_state();
            self.config.track_prefs = player.track_prefs();
            if window.is_fullscreen() {
                window.toggle_fullscreen();
            }
            set_video_background(false, window, cx);
        }
        self.log_out(window, cx);
        if let Screen::Login { view, .. } = &self.screen {
            view.update(cx, |login, cx| login.expired(&server, window, cx));
        }
    }

    fn log_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.config.session = None;
        save_config(&self.config, "session");
        self.screen = Self::login_screen(&self.config, window, cx);
        cx.notify();
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> TitleBar {
        let Screen::Main(main) = &self.screen else {
            return TitleBar::new();
        };
        // Search and Settings belong to no tab; Movie/Series details stay under theirs
        let off_tab = main.searching(cx)
            || (main.search_open && main.search.read(cx).has_recent())
            || main
                .details
                .last()
                .is_some_and(|d| d.view.entity_type() == std::any::TypeId::of::<SettingsView>());
        let selected = TABS
            .iter()
            .position(|(t, _)| *t == main.tab)
            .filter(|_| !off_tab);
        let this = cx.entity().downgrade();

        let user_name = main.session.user_name.clone();

        let search = if main.search_open || main.searching(cx) {
            div()
                .w(px(200.))
                .child(
                    Input::new(&main.search_input)
                        .small()
                        .cleanable(true)
                        .prefix(Icon::new(IconName::Search).small()),
                )
                .into_any_element()
        } else {
            Button::new("search")
                .ghost()
                .small()
                .icon(IconName::Search)
                .tooltip_with_action("Search", &FocusSearch, None)
                .on_click(
                    cx.listener(|this, _, window, cx| this.focus_search(&FocusSearch, window, cx)),
                )
                .into_any_element()
        };

        TitleBar::new()
            .h(px(40.))
            .border_b_0()
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .gap_1p5()
                    .child(
                        // mark's own aspect ratio (1707x1440), so it isn't squashed
                        svg()
                            .path("icons/tbis-mark.svg")
                            .w(px(19.))
                            .h(px(16.))
                            .text_color(cx.theme().primary),
                    )
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::BOLD)
                            .child("tbis"),
                    ),
            )
            // spans whole window width so tabs sit at true center; offset cancels TitleBar left padding
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(if cfg!(target_os = "macos") {
                        -80.
                    } else {
                        -12.
                    }))
                    .right_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        TabBar::new("nav")
                            .pill()
                            .small()
                            .children(TABS.map(|(_, label)| label))
                            .selected_index(selected.unwrap_or(usize::MAX))
                            .on_click(cx.listener(|this, index: &usize, window, cx| {
                                this.select_tab(*index, window, cx)
                            })),
                    ),
            )
            .child(
                h_flex().pr_2().gap_1().child(search).child(
                    Button::new("user-menu")
                        .ghost()
                        .small()
                        .child(initials(&user_name, cx))
                        .dropdown_caret(true)
                        .dropdown_menu(move |menu, _, _| {
                            let this = this.clone();
                            let settings = this.clone();
                            let updates = this.clone();
                            menu.label(user_name.clone())
                                .separator()
                                .item(PopupMenuItem::new("Settings").on_click(
                                    move |_, window, cx| {
                                        settings
                                            .update(cx, |this, cx| this.open_settings(window, cx))
                                            .ok();
                                    },
                                ))
                                .item(PopupMenuItem::new("Check for Updates\u{2026}").on_click(
                                    move |_, window, cx| {
                                        updates
                                            .update(cx, |this, cx| {
                                                this.check_for_updates(true, window, cx)
                                            })
                                            .ok();
                                    },
                                ))
                                .item(
                                    PopupMenuItem::new("Keyboard shortcuts")
                                        .on_click(|_, window, cx| shortcuts::open(window, cx)),
                                )
                                .item(PopupMenuItem::new("Log out").on_click(
                                    move |_, window, cx| {
                                        this.update(cx, |this, cx| this.log_out(window, cx)).ok();
                                    },
                                ))
                        }),
                ),
            )
    }
}

/// Player needs the window see-through so mpv's layer below gpui shows (ADR-0001).
/// Saves `config`, logging (not surfacing) a failure; `what` names the field for the log line.
fn save_config(config: &Config, what: &str) {
    if let Err(err) = config::save(config) {
        eprintln!("failed to save {what}: {err}");
    }
}

fn set_video_background(video: bool, window: &mut Window, cx: &mut App) {
    window.set_background_appearance(if video {
        WindowBackgroundAppearance::Transparent
    } else {
        WindowBackgroundAppearance::Opaque
    });
    if let Some(Some(root)) = window.root::<Root>() {
        root.update(cx, |root, cx| {
            root.style().background = video.then(|| hsla(0., 0., 0., 0.).into());
            cx.notify();
        });
    }
}

/// User menu badge: first two letters of the name on the theme's accent.
fn initials(name: &str, cx: &App) -> Div {
    div()
        .size_6()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(cx.theme().primary)
        .text_color(cx.theme().primary_foreground)
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .child(name.chars().take(2).collect::<String>().to_uppercase())
}

/// Fixed-width digits app-wide, so timecodes and counters don't jitter as they tick.
fn tabular_figures() -> FontFeatures {
    FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)]))
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Screen::Main(Main {
            player: Some((player, _)),
            ..
        }) = &self.screen
        {
            return div()
                .size_full()
                .font_features(tabular_figures())
                .on_action(|_: &ShowShortcuts, window, cx| shortcuts::open(window, cx))
                .child(player.clone())
                .children(Root::render_dialog_layer(window, cx))
                .children(Root::render_notification_layer(window, cx))
                .into_any_element();
        }

        let content = match &self.screen {
            Screen::Login { view, .. } => view.clone().into_any_element(),
            // focused and empty: the search view shows recent searches
            Screen::Main(main)
                if main.searching(cx)
                    || (main.search_open && main.search.read(cx).has_recent()) =>
            {
                main.search.clone().into_any_element()
            }
            Screen::Main(Main { details, .. }) if !details.is_empty() => {
                details[details.len() - 1].view.clone().into_any_element()
            }
            Screen::Main(Main {
                tab: Tab::Movies,
                movies,
                ..
            }) => movies.clone().into_any_element(),
            Screen::Main(Main {
                tab: Tab::Series,
                series,
                ..
            }) => series.clone().into_any_element(),
            Screen::Main(Main { home, .. }) => home.clone().into_any_element(),
        };

        v_flex()
            .size_full()
            .font_features(tabular_figures())
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::focus_search))
            .on_action(|_: &ShowShortcuts, window, cx| shortcuts::open(window, cx))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_title_bar(cx))
            .child(div().flex_1().min_h_0().child(content))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
            .into_any_element()
    }
}
