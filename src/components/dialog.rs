//! gpui-kit dialogs at the one radius.

use gpui_kit::component::text::TextView;
use gpui_kit::component::v_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::button::{VegaSize as _, ghost_button};
use crate::design::{gap, radius, sp};

/// Applies the vega frame to any gpui-kit Dialog or AlertDialog.
pub fn vega_dialog<D: Styled>(dialog: D) -> D {
    dialog.rounded(radius::base())
}

/// Release notes as scrollable markdown (none when empty), then a link to `page`.
pub fn release_notes(notes: SharedString, page: SharedString, cx: &App) -> Div {
    v_flex()
        .gap(gap::item())
        .when(!notes.trim().is_empty(), |this| {
            this.child(
                TextView::markdown("release-notes", notes)
                    .scrollable(true)
                    .max_h(sp(80.)),
            )
        })
        .child(
            ghost_button("release-notes-page", "View release on GitHub", cx)
                .sm()
                .on_click(move |_, _, cx| cx.open_url(&page)),
        )
}
