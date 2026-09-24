use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Root, Sizable as _, TitleBar, h_flex, v_flex,
};
use gpui_kit::*;

use crate::card::{OpenSeries, Play};
use crate::config::{self, Config};
use crate::jellyfin::{Api, Item, Kind, Session};
use crate::library::{LibraryView, SortChanged};
use crate::login::{LoggedIn, LoginView};
use crate::player::{Closed, PlayerView};
use crate::search::SearchView;
use crate::series::{Back, SeriesView};

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
    movies: Entity<LibraryView>,
    series: Entity<LibraryView>,
    search_input: Entity<InputState>,
    search: Entity<SearchView>,
    /// Series detail shown over the current tab.
    detail: Option<(Entity<SeriesView>, [Subscription; 2])>,
    player: Option<(Entity<PlayerView>, Subscription)>,
    _subscriptions: [Subscription; 7],
}

impl Main {
    fn searching(&self, cx: &App) -> bool {
        !self.search_input.read(cx).value().trim().is_empty()
    }

    /// Reloads whatever is on screen so played state and progress are fresh.
    fn refresh_visible(&self, cx: &mut App) {
        if let Some((detail, _)) = &self.detail {
            detail.update(cx, |detail, cx| detail.refresh(cx));
        } else {
            match self.tab {
                Tab::Movies => self.movies.update(cx, |view, cx| view.refresh(cx)),
                Tab::Series => self.series.update(cx, |view, cx| view.refresh(cx)),
                Tab::Home => {}
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
        Self {
            config,
            screen,
            focus,
        }
    }

    fn login_screen(config: &Config, window: &mut Window, cx: &mut Context<Self>) -> Screen {
        let login = cx.new(|cx| LoginView::new(config.device_id.clone(), window, cx));
        let subscription =
            cx.subscribe_in(&login, window, |this, _, LoggedIn(session), window, cx| {
                this.config.session = Some(session.clone());
                if let Err(err) = config::save(&this.config) {
                    eprintln!("failed to save session: {err}");
                }
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
        let api = Api::new(cx.http_client(), session.clone(), config.device_id.clone());
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
                if let Err(err) = config::save(&this.config) {
                    eprintln!("failed to save sort: {err}");
                }
            }),
            cx.subscribe(&series, |this, _, SortChanged(sort), _| {
                this.config.series_sort = *sort;
                if let Err(err) = config::save(&this.config) {
                    eprintln!("failed to save sort: {err}");
                }
            }),
            cx.subscribe_in(&movies, window, |this, _, Play(item), window, cx| {
                this.open_player(item, window, cx)
            }),
            cx.subscribe_in(&series, window, |this, _, OpenSeries(item), window, cx| {
                this.open_series(item, window, cx)
            }),
            cx.subscribe_in(&search, window, |this, _, Play(item), window, cx| {
                this.open_player(item, window, cx)
            }),
            cx.subscribe_in(&search, window, |this, _, OpenSeries(item), window, cx| {
                this.open_series(item, window, cx)
            }),
            cx.subscribe(&search_input, |this, input, event: &InputEvent, cx| {
                if let (InputEvent::Change, Screen::Main(main)) = (event, &this.screen) {
                    let query = input.read(cx).value();
                    main.search
                        .update(cx, |search, cx| search.set_query(&query, cx));
                    cx.notify();
                }
            }),
        ];
        Screen::Main(Main {
            session,
            api,
            tab: Tab::Home,
            movies,
            series,
            search_input,
            search,
            detail: None,
            player: None,
            _subscriptions,
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

    fn open_series(&mut self, series: &Item, window: &mut Window, cx: &mut Context<Self>) {
        let Screen::Main(main) = &mut self.screen else {
            return;
        };
        let (api, series) = (main.api.clone(), series.clone());
        let detail = cx.new(|cx| SeriesView::new(api, series, cx));
        let subscriptions = [
            cx.subscribe_in(&detail, window, |this, _, Play(item), window, cx| {
                this.open_player(item, window, cx)
            }),
            cx.subscribe(&detail, |this, _, Back, cx| {
                if let Screen::Main(main) = &mut this.screen {
                    main.detail = None;
                    main.refresh_visible(cx);
                    cx.notify();
                }
            }),
        ];
        main.detail = Some((detail, subscriptions));
        Self::clear_search(main, window, cx);
        cx.notify();
    }

    fn focus_search(&mut self, _: &FocusSearch, window: &mut Window, cx: &mut Context<Self>) {
        if let Screen::Main(main) = &self.screen
            && main.player.is_none()
        {
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
        let player = cx.new(|cx| PlayerView::new(api, item, volume, window, cx));
        let subscription =
            cx.subscribe_in(&player, window, |this, _, closed: &Closed, window, cx| {
                this.config.volume = closed.volume;
                this.config.muted = closed.muted;
                if let Err(err) = config::save(&this.config) {
                    eprintln!("failed to save volume: {err}");
                }
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

    fn log_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.config.session = None;
        if let Err(err) = config::save(&self.config) {
            eprintln!("failed to clear session: {err}");
        }
        self.screen = Self::login_screen(&self.config, window, cx);
        cx.notify();
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> TitleBar {
        let Screen::Main(main) = &self.screen else {
            return TitleBar::new();
        };
        let selected = TABS.iter().position(|(t, _)| *t == main.tab).unwrap_or(0);
        let this = cx.entity().downgrade();

        TitleBar::new()
            .child(
                TabBar::new("nav")
                    .segmented()
                    .small()
                    .children(TABS.map(|(_, label)| label))
                    .selected_index(selected)
                    .on_click(cx.listener(|this, index: &usize, window, cx| {
                        this.select_tab(*index, window, cx)
                    })),
            )
            .child(
                h_flex()
                    .pr_2()
                    .gap_2()
                    .child(
                        div().w(px(220.)).child(
                            Input::new(&main.search_input)
                                .small()
                                .cleanable(true)
                                .prefix(Icon::new(IconName::Search).small()),
                        ),
                    )
                    .child(
                        Button::new("user-menu")
                            .ghost()
                            .small()
                            .label(main.session.user_name.clone())
                            .dropdown_caret(true)
                            .dropdown_menu(move |menu, _, _| {
                                let this = this.clone();
                                menu.item(PopupMenuItem::new("Log out").on_click(
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Screen::Main(Main {
            player: Some((player, _)),
            ..
        }) = &self.screen
        {
            return div().size_full().child(player.clone()).into_any_element();
        }

        let content = match &self.screen {
            Screen::Login { view, .. } => view.clone().into_any_element(),
            Screen::Main(main) if main.searching(cx) => main.search.clone().into_any_element(),
            Screen::Main(Main {
                detail: Some((detail, _)),
                ..
            }) => detail.clone().into_any_element(),
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
            Screen::Main(Main { tab, .. }) => {
                let title = TABS
                    .iter()
                    .find(|(t, _)| t == tab)
                    .map_or("", |(_, label)| label);
                div()
                    .p_6()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title)
                    .into_any_element()
            }
        };

        v_flex()
            .size_full()
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::focus_search))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_title_bar(cx))
            .child(div().flex_1().min_h_0().child(content))
            .into_any_element()
    }
}
