use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{Root, WindowExt as _};
use gpui_kit::*;

use crate::backdrops::Backdrops;
use crate::components::layout::{grow, screen};
use crate::config::{self, Config, WindowState};
use crate::design;
use crate::home::HomeView;
use crate::jellyfin::{Api, Item, Kind, Session};
use crate::library::{LibraryView, SortChanged};
use crate::login::{LoggedIn, LoginView};
use crate::movie::MovieView;
use crate::nav::Nav;
use crate::player::{Closed, PlayerView};
use crate::search::{SearchEvent, SearchView};
use crate::series::SeriesView;
use crate::settings::{Settings, SettingsChanged, SettingsView};
use crate::shortcuts::{self, ShowShortcuts};
use crate::stats::StatsPage;
mod title_bar;
mod updates;

actions!(
    tbis,
    [FocusSearch, GoBack, ShowHome, ShowMovies, ShowSeries]
);

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
    /// Library backdrop behind Home, Movies and Series (Settings > Backdrops).
    backdrops: Entity<Backdrops>,
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

    /// Reloads whatever is on screen so played state and progress are fresh; marks every
    /// tab stale so the next visit reloads it too.
    fn refresh_visible(&self, cx: &mut App) {
        self.home.update(cx, |view, _| view.mark_stale());
        self.movies.update(cx, |view, _| view.mark_stale());
        self.series.update(cx, |view, _| view.mark_stale());
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

    /// Tab visit: reloads the tab only when its data is stale.
    fn refresh_tab_if_stale(&self, cx: &mut App) {
        match self.tab {
            Tab::Movies => self.movies.update(cx, |view, cx| view.refresh_if_stale(cx)),
            Tab::Series => self.series.update(cx, |view, cx| view.refresh_if_stale(cx)),
            Tab::Home => self.home.update(cx, |view, cx| view.refresh_if_stale(cx)),
        }
    }

    /// Points the page backdrop at the current tab's pool; nothing loads while `on` is false.
    fn sync_backdrops(&self, on: bool, cx: &mut App) {
        if !on {
            return;
        }
        let kind = match self.tab {
            Tab::Home => None,
            Tab::Movies => Some(Kind::Movie),
            Tab::Series => Some(Kind::Series),
        };
        self.backdrops
            .update(cx, |backdrops, cx| backdrops.show(kind, cx));
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
    /// Cancels pending series Play lookups when their navigation context changes.
    navigation_generation: u64,
    navigation_lookup: Task<()>,
}

impl AppView {
    pub fn new(config: Config, window: &mut Window, cx: &mut Context<Self>) -> Self {
        if let Some(err) = config::load_error(&config) {
            window.push_notification(
                Notification::error(format!(
                    "Could not load saved configuration ({err}). It was not overwritten; fix or move config.json to recover."
                )),
                cx,
            );
        }
        cx.set_global(config.seek);
        cx.set_global(config.hide_spoilers);
        cx.set_global(config.auto_skip);
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
            navigation_generation: 0,
            navigation_lookup: Task::ready(()),
        };
        this.check_for_updates(false, window, cx);
        this
    }

    fn login_screen(config: &Config, window: &mut Window, cx: &mut Context<Self>) -> Screen {
        let login = cx.new(|cx| LoginView::new(config.device_id.clone(), window, cx));
        let subscription =
            cx.subscribe_in(&login, window, |this, _, LoggedIn(session), window, cx| {
                this.invalidate_navigation();
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
        let server = session.server.clone();
        let api = Api::new(
            cx.http_client(),
            session.clone(),
            server,
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
            let mut home = HomeView::new(api.clone(), config.home_layout);
            home.refresh(cx);
            home
        });
        let grouping = config.library_grouping;
        let movies =
            cx.new(|_| LibraryView::new(Kind::Movie, api.clone(), config.movies_sort, grouping));
        let series =
            cx.new(|_| LibraryView::new(Kind::Series, api.clone(), config.series_sort, grouping));
        let local = [movies.clone(), series.clone()];
        let recent = config.recent_searches.clone();
        let search = cx.new(|_| SearchView::new(api.clone(), local, recent));
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search")
                .clean_on_escape()
        });
        let backdrops = cx.new(|cx| Backdrops::new(api.clone(), cx));
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
        let main = Main {
            session,
            api,
            tab: Tab::Home,
            home,
            movies,
            series,
            search_input,
            search,
            backdrops,
            search_open: false,
            details: Vec::new(),
            player: None,
            _subscriptions,
            _expired,
        };
        main.sync_backdrops(config.backdrops.0, cx);
        Screen::Main(main)
    }

    fn invalidate_navigation(&mut self) {
        self.navigation_generation = self.navigation_generation.wrapping_add(1);
        self.navigation_lookup = Task::ready(());
    }

    fn select_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.invalidate_navigation();
        let Screen::Main(main) = &mut self.screen else {
            return;
        };
        main.tab = TABS[index].0;
        main.sync_backdrops(self.config.backdrops.0, cx);
        main.details.clear();
        Self::clear_search(main, window, cx);
        main.refresh_tab_if_stale(cx);
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
        self.invalidate_navigation();
        let generation = self.navigation_generation;
        match nav {
            Nav::Open(item) => match item.kind {
                Kind::Series => self.open_series(item, window, cx),
                // Episode detail shares the Movie page; Other would be a server data bug
                Kind::Movie | Kind::Episode | Kind::Other => self.open_movie(item, window, cx),
            },
            Nav::Play(item) if item.kind == Kind::Series => {
                let Screen::Main(main) = &self.screen else {
                    return;
                };
                let (api, series) = (main.api.clone(), item.clone());
                self.navigation_lookup = cx.spawn_in(window, async move |this, cx| {
                    let next = api.next_up(&series.id).await;
                    this.update_in(cx, |this, window, cx| {
                        let player_open =
                            matches!(&this.screen, Screen::Main(m) if m.player.is_some());
                        let current = this.navigation_generation;
                        match series_play(generation, current, player_open, next) {
                            SeriesPlay::Ignore => {}
                            SeriesPlay::Play(episode) => this.open_player(&episode, window, cx),
                            SeriesPlay::OpenSeries => this.open_series(&series, window, cx),
                        }
                    })
                    .ok();
                });
            }
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

    /// Pops one detail page; with none open, clears an active search.
    fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &mut self.screen else {
            return;
        };
        if !main.details.is_empty() {
            self.on_nav(&Nav::Back, window, cx);
        } else if main.search_open || main.searching(cx) {
            main.search_open = false;
            Self::clear_search(main, window, cx);
            cx.notify();
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
        self.invalidate_navigation();
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
        let view = cx.new(|cx| SettingsView::new(&self.config, window, cx));
        let changed = cx.subscribe(&view, |this, _, changed: &SettingsChanged, cx| {
            let new = &changed.0;
            let old = Settings::from_config(&this.config);
            new.write_to(&mut this.config);
            cx.set_global(new.seek);
            cx.set_global(new.hide_spoilers);
            cx.set_global(new.auto_skip);
            if let Screen::Main(main) = &this.screen {
                if new.backdrops != old.backdrops {
                    main.sync_backdrops(new.backdrops.0, cx);
                }
                if new.home_layout != old.home_layout {
                    main.home
                        .update(cx, |view, cx| view.set_layout(new.home_layout, cx));
                }
                if new.library_grouping != old.library_grouping {
                    for view in [&main.movies, &main.series] {
                        view.update(cx, |view, cx| view.set_grouping(new.library_grouping, cx));
                    }
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

    fn open_stats(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &self.screen else {
            return;
        };
        let api = main.api.clone();
        let view = cx.new(|cx| StatsPage::new(api, cx));
        self.set_detail(view, |_| {}, window, cx);
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
        if matches!(&self.screen, Screen::Main(main) if main.player.is_none()) {
            self.invalidate_navigation();
        }
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
        self.invalidate_navigation();
        let persistence = config::forget_session(&mut self.config);
        self.screen = Self::login_screen(&self.config, window, cx);
        if let Err(message) = persistence {
            window.push_notification(Notification::error(message), cx);
        }
        cx.notify();
    }
}

/// What a finished series Play lookup opens; `E` is the Next Up Episode.
enum SeriesPlay<E> {
    /// Its navigation context is gone, or a Player is already up.
    Ignore,
    Play(E),
    OpenSeries,
}

/// `lookup`: the navigation generation the lookup started in; any navigation, login or
/// logout since then moved `current` on. `player_open`: a slow lookup or a double click
/// lost to a Player that is already up.
fn series_play<E>(
    lookup: u64,
    current: u64,
    player_open: bool,
    next: anyhow::Result<Option<E>>,
) -> SeriesPlay<E> {
    if lookup != current || player_open {
        return SeriesPlay::Ignore;
    }
    match next {
        Ok(Some(episode)) => SeriesPlay::Play(episode),
        Ok(None) => SeriesPlay::OpenSeries,
        Err(err) => {
            crate::log::write(&format!("next up lookup failed: {err}"));
            SeriesPlay::OpenSeries
        }
    }
}

/// Saves `config`, logging (not surfacing) a failure; `what` names the field for the log line.
fn save_config(config: &Config, what: &str) {
    if let Err(err) = config::save(config) {
        crate::log::write(&format!("failed to save {what}: {err}"));
    }
}

/// Player needs the window see-through so mpv's layer below gpui shows (ADR-0001).
fn set_video_background(video: bool, window: &mut Window, cx: &mut App) {
    window.set_background_appearance(if video {
        WindowBackgroundAppearance::Transparent
    } else {
        WindowBackgroundAppearance::Opaque
    });
    if let Some(Some(root)) = window.root::<Root>() {
        root.update(cx, |root, cx| {
            root.style().background = video.then(|| design::overlay::CLEAR.into());
            cx.notify();
        });
    }
}

/// Fixed-width digits app-wide, so timecodes and counters don't jitter as they tick.
fn tabular_figures() -> FontFeatures {
    FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)]))
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                .into_any_element();
        }

        // focused and empty: the search view shows recent searches
        let search_shown = matches!(&self.screen, Screen::Main(main)
            if main.searching(cx) || (main.search_open && main.search.read(cx).has_recent()));
        let backdrop = match &self.screen {
            Screen::Main(main)
                if self.config.backdrops.0 && main.details.is_empty() && !search_shown =>
            {
                Some(main.backdrops.clone())
            }
            _ => None,
        };
        let content = match &self.screen {
            Screen::Login { view, .. } => view.clone().into_any_element(),
            Screen::Main(main) if search_shown => main.search.clone().into_any_element(),
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

        screen(cx)
            .font_features(tabular_figures())
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::focus_search))
            .on_action(cx.listener(|this, _: &GoBack, window, cx| this.go_back(window, cx)))
            .on_action(cx.listener(|this, _: &ShowHome, window, cx| this.select_tab(0, window, cx)))
            .on_action(
                cx.listener(|this, _: &ShowMovies, window, cx| this.select_tab(1, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ShowSeries, window, cx| this.select_tab(2, window, cx)),
            )
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Back),
                cx.listener(|this, _, window, cx| this.go_back(window, cx)),
            )
            .on_action(|_: &ShowShortcuts, window, cx| shortcuts::open(window, cx))
            .child(self.render_title_bar(cx))
            .child(grow().relative().children(backdrop).child(content))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{SeriesPlay, series_play};

    fn episode() -> &'static str {
        "episode"
    }

    fn ignored(play: SeriesPlay<&str>) -> bool {
        matches!(play, SeriesPlay::Ignore)
    }

    #[test]
    fn a_current_lookup_plays_next_up_or_falls_back_to_the_series() {
        let play = series_play(3, 3, false, Ok(Some(episode())));
        assert!(matches!(play, SeriesPlay::Play("episode")));
        let none = series_play::<&str>(3, 3, false, Ok(None));
        assert!(matches!(none, SeriesPlay::OpenSeries));
        let failed = series_play::<&str>(3, 3, false, Err(anyhow::anyhow!("offline")));
        assert!(matches!(failed, SeriesPlay::OpenSeries));
    }

    #[test]
    fn a_lookup_resolving_after_navigation_or_a_new_session_does_nothing() {
        // navigating away, logging out and logging in each move the generation on
        for current in [4, 5, 6] {
            assert!(ignored(series_play(3, current, false, Ok(Some(episode())))));
            assert!(ignored(series_play(3, current, false, Ok(None))));
            let failed = series_play(3, current, false, Err(anyhow::anyhow!("401")));
            assert!(ignored(failed));
        }
    }

    #[test]
    fn a_competing_play_wins_over_an_older_lookup() {
        // a second Play moved the generation on, or already opened its Player
        assert!(ignored(series_play(3, 4, false, Ok(Some(episode())))));
        assert!(ignored(series_play(3, 3, true, Ok(Some(episode())))));
        assert!(ignored(series_play(3, 3, true, Ok(None))));
    }
}
