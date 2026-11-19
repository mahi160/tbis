//! Title bar for `AppView`.

use super::*;
use crate::components::button::{avatar_button, surface_icon_button};
use crate::components::input::search_input;
use crate::components::tabs::tabs;
use crate::components::title_bar::{app_title_bar, title_bar};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{IconName, TitleBar};

impl AppView {
    pub(super) fn render_title_bar(&self, cx: &mut Context<Self>) -> TitleBar {
        let Screen::Main(main) = &self.screen else {
            return title_bar(cx);
        };
        // Search, Settings and Watch stats belong to no tab; Movie/Series details stay under theirs
        let off_tab = main.searching(cx)
            || (main.search_open && main.search.read(cx).has_recent())
            || main.details.last().is_some_and(|d| {
                let page = d.view.entity_type();
                page == std::any::TypeId::of::<SettingsView>()
                    || page == std::any::TypeId::of::<StatsPage>()
            });
        let selected = TABS
            .iter()
            .position(|(t, _)| *t == main.tab)
            .filter(|_| !off_tab);
        let this = cx.entity().downgrade();

        let user_name = main.session.user_name.clone();

        let search = if main.search_open || main.searching(cx) {
            search_input(&main.search_input, cx).into_any_element()
        } else {
            surface_icon_button("search", IconName::Search, "Search", cx)
                .tooltip_with_action("Search", &FocusSearch, None)
                .on_click(
                    cx.listener(|this, _, window, cx| this.focus_search(&FocusSearch, window, cx)),
                )
                .into_any_element()
        };

        let nav = tabs(
            "nav",
            TABS.map(|(_, label)| label),
            selected.unwrap_or(usize::MAX),
            cx.listener(|this, index: &usize, window, cx| this.select_tab(*index, window, cx)),
        );
        let user_menu = avatar_button("user-menu", user_name.clone(), cx)
            .tooltip("Account")
            .dropdown_menu(move |menu, _, _| {
                let this = this.clone();
                let settings = this.clone();
                let stats = this.clone();
                let updates = this.clone();
                menu.label(user_name.clone())
                    .separator()
                    .item(
                        PopupMenuItem::new("Settings").on_click(move |_, window, cx| {
                            settings
                                .update(cx, |this, cx| this.open_settings(window, cx))
                                .ok();
                        }),
                    )
                    .item(
                        PopupMenuItem::new("Watch stats").on_click(move |_, window, cx| {
                            stats
                                .update(cx, |this, cx| this.open_stats(window, cx))
                                .ok();
                        }),
                    )
                    .item(PopupMenuItem::new("Check for Updates\u{2026}").on_click(
                        move |_, window, cx| {
                            updates
                                .update(cx, |this, cx| this.check_for_updates(true, window, cx))
                                .ok();
                        },
                    ))
                    .item(
                        PopupMenuItem::new("Keyboard shortcuts")
                            .on_click(|_, window, cx| shortcuts::open(window, cx)),
                    )
                    .item(
                        PopupMenuItem::new("Log out").on_click(move |_, window, cx| {
                            this.update(cx, |this, cx| this.log_out(window, cx)).ok();
                        }),
                    )
            });
        app_title_bar(nav, [search, user_menu.into_any_element()], cx)
    }
}
