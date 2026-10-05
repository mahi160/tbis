//! Keyboard shortcut list: grouped rows of action label and key caps. Key caps are
//! gpui-kit Kbd overridden to vega's `h-5 min-w-5 rounded-sm bg-muted px-1 text-xs
//! font-medium text-muted-foreground`; groups are vega Field groups (`gap-3`).

use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use super::typography::subheading;
use crate::design::{color, radius, sp, text};

/// One group: its heading and `(label, keys)` rows.
pub type ShortcutGroup = (&'static str, Vec<(String, Vec<Kbd>)>);

fn key_cap(key: Kbd, cx: &App) -> Kbd {
    let recipe = crate::design::tokens().kbd;
    Styled::h(key, sp(recipe.size))
        .min_w(sp(recipe.size))
        .px(sp(recipe.px))
        .rounded(radius::sm())
        .bg(color::muted(cx))
        .text_color(color::muted_foreground(cx))
        .text_size(text::xs())
        .font_weight(FontWeight::MEDIUM)
}

/// Scrolling list of groups, label left and key caps right.
pub fn shortcut_list(groups: Vec<ShortcutGroup>, cx: &App) -> Stateful<Div> {
    v_flex()
        .id("shortcuts")
        .max_h(sp(130.))
        .overflow_y_scroll()
        .gap(sp(6.))
        .children(groups.into_iter().map(|(group, rows)| {
            v_flex()
                .gap(sp(3.))
                .child(subheading(group).text_color(color::muted_foreground(cx)))
                .children(rows.into_iter().map(|(label, keys)| {
                    h_flex()
                        .justify_between()
                        .text_size(text::sm())
                        .child(label)
                        .child(
                            h_flex()
                                .gap(sp(1.))
                                .children(keys.into_iter().map(|key| key_cap(key, cx))),
                        )
                }))
        }))
}
