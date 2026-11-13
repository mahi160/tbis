//! Single-choice dropdown (gpui-kit DropdownMenu): a checkmark on the current option,
//! optionally with a second list of choices in a submenu.

use std::rc::Rc;

use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
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
    let on_pick = Rc::new(on_pick);
    button
        .dropdown_menu(move |menu, _, _| {
            let on_pick = on_pick.clone();
            choices(menu.scrollable(scrollable), &options, move |ix, cx| {
                on_pick(ix, cx)
            })
        })
        .on_open_change(on_open)
        .into_any_element()
}

/// A labelled `(label, checked)` list opened as a submenu.
pub type Submenu = (SharedString, Vec<(SharedString, bool)>);

/// `choice_menu` whose last row opens `submenu`, a second list of choices; `on_pick` gets
/// the list (0: `options`, 1: `submenu`) and the index within it.
pub fn choice_menu_with_submenu(
    button: Button,
    options: Vec<(SharedString, bool)>,
    submenu: Submenu,
    scrollable: bool,
    on_pick: impl Fn(usize, usize, &mut App) + 'static,
    on_open: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let on_pick = Rc::new(on_pick);
    let (label, nested) = (submenu.0, Rc::new(submenu.1));
    button
        .dropdown_menu(move |menu, window, cx| {
            let main = on_pick.clone();
            let menu = choices(menu.scrollable(scrollable), &options, move |ix, cx| {
                main(0, ix, cx)
            })
            .separator();
            let (on_pick, nested) = (on_pick.clone(), nested.clone());
            menu.submenu(label.clone(), window, cx, move |menu, _, _| {
                let on_pick = on_pick.clone();
                choices(menu.scrollable(scrollable), &nested, move |ix, cx| {
                    on_pick(1, ix, cx)
                })
            })
        })
        .on_open_change(on_open)
        .into_any_element()
}

/// Adds `options` to `menu`, each checked as given; `on_pick` gets the picked index.
fn choices(
    mut menu: PopupMenu,
    options: &[(SharedString, bool)],
    on_pick: impl Fn(usize, &mut App) + 'static,
) -> PopupMenu {
    let on_pick = Rc::new(on_pick);
    for (ix, (label, checked)) in options.iter().enumerate() {
        let on_pick = on_pick.clone();
        menu = menu.item(
            PopupMenuItem::new(label.clone())
                .checked(*checked)
                .on_click(move |_, _, cx| on_pick(ix, cx)),
        );
    }
    menu
}
