use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, TitleBar, h_flex, v_flex};
use gpui_kit::*;

use crate::config::{self, Config};
use crate::jellyfin::{Api, Session};
use crate::login::{LoggedIn, LoginView};
use crate::movies::{MoviesView, SortChanged};

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

enum Screen {
    Login {
        view: Entity<LoginView>,
        _subscription: Subscription,
    },
    Main {
        session: Session,
        tab: Tab,
        movies: Entity<MoviesView>,
        _subscription: Subscription,
    },
}

pub struct AppView {
    config: Config,
    screen: Screen,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = config::load();
        let screen = match config.session.clone() {
            Some(session) => Self::main_screen(session, &config, cx),
            None => Self::login_screen(&config, window, cx),
        };
        Self { config, screen }
    }

    fn login_screen(config: &Config, window: &mut Window, cx: &mut Context<Self>) -> Screen {
        let login = cx.new(|cx| LoginView::new(config.device_id.clone(), window, cx));
        let subscription = cx.subscribe(&login, |this, _, LoggedIn(session), cx| {
            this.config.session = Some(session.clone());
            if let Err(err) = config::save(&this.config) {
                eprintln!("failed to save session: {err}");
            }
            this.screen = Self::main_screen(session.clone(), &this.config, cx);
            cx.notify();
        });
        Screen::Login {
            view: login,
            _subscription: subscription,
        }
    }

    fn main_screen(session: Session, config: &Config, cx: &mut Context<Self>) -> Screen {
        let api = Api::new(cx.http_client(), session.clone(), config.device_id.clone());
        let movies = cx.new(|_| MoviesView::new(api, config.movies_sort));
        let subscription = cx.subscribe(&movies, |this, _, SortChanged(sort), _| {
            this.config.movies_sort = *sort;
            if let Err(err) = config::save(&this.config) {
                eprintln!("failed to save sort: {err}");
            }
        });
        Screen::Main {
            session,
            tab: Tab::Home,
            movies,
            _subscription: subscription,
        }
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
        let Screen::Main { session, tab, .. } = &self.screen else {
            return TitleBar::new();
        };
        let selected = TABS.iter().position(|(t, _)| t == tab).unwrap_or(0);
        let this = cx.entity().downgrade();

        TitleBar::new()
            .child(
                TabBar::new("nav")
                    .segmented()
                    .small()
                    .children(TABS.map(|(_, label)| label))
                    .selected_index(selected)
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        if let Screen::Main { tab, movies, .. } = &mut this.screen {
                            *tab = TABS[*index].0;
                            if *tab == Tab::Movies {
                                movies.update(cx, |movies, cx| movies.refresh(cx));
                            }
                            cx.notify();
                        }
                    })),
            )
            .child(
                h_flex().pr_2().child(
                    Button::new("user-menu")
                        .ghost()
                        .small()
                        .label(session.user_name.clone())
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

impl Render for AppView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match &self.screen {
            Screen::Login { view, .. } => view.clone().into_any_element(),
            Screen::Main {
                tab: Tab::Movies,
                movies,
                ..
            } => movies.clone().into_any_element(),
            Screen::Main { tab, .. } => {
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
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_title_bar(cx))
            .child(div().flex_1().min_h_0().child(content))
    }
}
