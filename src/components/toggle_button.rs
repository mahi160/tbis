//! Watched/Favorite icon button; `selected` while the flag is set.

use gpui_kit::component::Icon;
use gpui_kit::component::button::Button;
use gpui_kit::*;

use super::button::{GhostState as _, icon_button};
use crate::jellyfin::UserData;

/// Watched/Favorite flag a detail page can flip on one of its items.
#[derive(Clone, Copy)]
pub enum Toggle {
    Played,
    Favorite,
}

pub fn toggle_button(
    id: impl Into<ElementId>,
    toggle: Toggle,
    data: &UserData,
    cx: &App,
) -> Button {
    let (icon, on, label_on, label_off) = match toggle {
        Toggle::Played => (
            assets::IconName::CircleCheck,
            data.played,
            "Mark unwatched",
            "Mark watched",
        ),
        Toggle::Favorite => (
            assets::IconName::Heart,
            data.is_favorite,
            "Remove from favorites",
            "Add to favorites",
        ),
    };
    icon_button(
        id,
        Icon::new(icon),
        if on { label_on } else { label_off },
        cx,
    )
    .ghost_selected(on, cx)
}
