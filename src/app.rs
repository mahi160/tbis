use gpui_kit::component::avatar::Avatar;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Root, Sizable as _, TitleBar, h_flex, v_flex,
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
use crate::search::SearchView;
use crate::series::SeriesView;
use crate::settings::{LanguageChanged, SettingsView};

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
    /// Movie or Series detail shown over the current tab.
    detail: Option<Detail>,
    player: Option<(Entity<PlayerView>, Subscription)>,
    _subscriptions: [Subscription; 7],
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

impl Main {
    fn searching(&self, cx: &App) -> bool {
        !self.search_input.read(cx).value().trim().is_empty()
    }

    /// Reloads whatever is on screen so played state and progress are fresh.
    fn refresh_visible(&self, cx: &mut App) {
        if let Some(detail) = &self.detail {
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
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = config::load();
        let screen = match config.session.clone() {
            Some(session) => Self::main_screen(session, &config, window, cx),
            None => Self::login_screen(&config, window, cx),
        };
        let focus = cx.focus_handle();
        if matches!(screen, Screen::Main(_)) {
            focus.focus(window, cx);
        }
        cx.on_app_quit(|this, cx| {
            // cmd-Q/window close skip Player::close, so its report never fires and the
            // volume never gets saved; do both here with the shutdown budget instead
            let mut report = None;
            if let Screen::Main(main) = &this.screen
                && let Some((player, _)) = &main.player
            {
                let (volume, muted) = player.read(cx).volume_state();
                this.config.volume = volume;
                this.config.muted = muted;
                this.config.track_prefs = player.read(cx).track_prefs();
                report = player.read(cx).quit_report();
            }
            save_config(&this.config, "volume/window");
            async move {
                if let Some(report) = report {
                    report.await;
                }
            }
        })
        .detach();
        let _window_bounds = cx.observe_window_bounds(window, |this, window, _| {
            this.config.window = Some(WindowState::from(window.window_bounds()));
        });
        Self {
            config,
            screen,
            focus,
            _window_bounds,
        }
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
        let search = cx.new(|_| SearchView::new(api.clone()));
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
            cx.subscribe_in(&search, window, |this, _, nav, window, cx| {
                this.on_nav(nav, window, cx)
            }),
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
            detail: None,
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
        main.detail = None;
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
                // an Episode or Other reaching Nav::Open would be a server data bug
                Kind::Movie | Kind::Episode | Kind::Other => self.open_movie(item, window, cx),
            },
            Nav::Play(item) => self.open_player(item, window, cx),
            Nav::Back => {
                let Screen::Main(main) = &mut self.screen else {
                    return;
                };
                main.detail = None;
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
        main.detail = Some(Detail {
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
        let language = self.config.language.clone();
        let view = cx.new(|cx| SettingsView::new(language, cx));
        let changed = cx.subscribe(&view, |this, _, LanguageChanged(language), _| {
            this.config.language = language.clone();
            save_config(&this.config, "language");
        });
        self.set_detail(view, |_| {}, window, cx);
        if let Screen::Main(Main {
            detail: Some(detail),
            ..
        }) = &mut self.screen
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
        let volume = (self.config.volume, self.config.muted);
        let track_prefs = self.config.track_prefs.clone();
        let language = self.config.language.clone();
        let player =
            cx.new(|cx| PlayerView::new(api, item, volume, track_prefs, language, window, cx));
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
        let selected = TABS.iter().position(|(t, _)| *t == main.tab).unwrap_or(0);
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
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("tbis"),
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
                            .underline()
                            .small()
                            .children(TABS.map(|(_, label)| label))
                            .selected_index(selected)
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
                        .child(Avatar::new().name(user_name.clone()).small())
                        .dropdown_caret(true)
                        .dropdown_menu(move |menu, _, _| {
                            let this = this.clone();
                            let settings = this.clone();
                            menu.label(user_name.clone())
                                .separator()
                                .item(PopupMenuItem::new("Settings").on_click(
                                    move |_, window, cx| {
                                        settings
                                            .update(cx, |this, cx| this.open_settings(window, cx))
                                            .ok();
                                    },
                                ))
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

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Screen::Main(Main {
            player: Some((player, _)),
            ..
        }) = &self.screen
        {
            return div()
                .size_full()
                .child(player.clone())
                .children(Root::render_notification_layer(window, cx))
                .into_any_element();
        }

        let content = match &self.screen {
            Screen::Login { view, .. } => view.clone().into_any_element(),
            Screen::Main(main) if main.searching(cx) => main.search.clone().into_any_element(),
            Screen::Main(Main {
                detail: Some(detail),
                ..
            }) => detail.view.clone().into_any_element(),
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
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::focus_search))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_title_bar(cx))
            .child(div().flex_1().min_h_0().child(content))
            .children(Root::render_notification_layer(window, cx))
            .into_any_element()
    }
}
