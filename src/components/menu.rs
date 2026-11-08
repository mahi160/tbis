//! Single-choice dropdown (gpui-kit DropdownMenu): a checkmark on the current option.

use std::rc::Rc;

use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::*;

/// Opens from `button`; `options` are `(label, checked)`; `on_pick` gets the index.
/// `scrollable` for long lists that would overflow the window; `on_open` hears open/close.
pub fn choice_menu(
    button: Button,
    options: Vec<(SharedString, bool)>,
    scrollable: bool,
    on_pick: impl Fn(usize, &mut App) + 'static,
    on_open: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    choice_menu_sections(
        button,
        vec![(None, options)],
        scrollable,
        move |_, ix, cx| on_pick(ix, cx),
        on_open,
    )
}

/// An optional heading and its `(label, checked)` options.
pub type MenuSection = (Option<SharedString>, Vec<(SharedString, bool)>);

/// `choice_menu` with several lists in one dropdown, each under an optional heading;
/// `on_pick` gets the section and the index within it.
pub fn choice_menu_sections(
    button: Button,
    sections: Vec<MenuSection>,
    scrollable: bool,
    on_pick: impl Fn(usize, usize, &mut App) + 'static,
    on_open: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let on_pick = Rc::new(on_pick);
    button
        .dropdown_menu(move |menu, _, _| {
            let mut menu = menu.scrollable(scrollable);
            for (section, (heading, options)) in sections.iter().enumerate() {
                if section > 0 {
                    menu = menu.item(PopupMenuItem::separator());
                }
                if let Some(heading) = heading {
                    menu = menu.item(PopupMenuItem::label(heading.clone()));
                }
                for (ix, (label, checked)) in options.iter().enumerate() {
                    let on_pick = on_pick.clone();
                    menu = menu.item(
                        PopupMenuItem::new(label.clone())
                            .checked(*checked)
                            .on_click(move |_, _, cx| on_pick(section, ix, cx)),
                    );
                }
            }
            menu
        })
        .on_open_change(on_open)
        .into_any_element()
}
