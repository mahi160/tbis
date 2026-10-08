//! Keyboard shortcut list: grouped rows of action label and key caps. Key caps are
//! gpui-kit Kbd overridden to vega's `bg-muted font-medium text-muted-foreground` at the
//! recipe size, the one radius and meta text.

use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use super::typography::subheading;
use crate::design::{color, gap, radius, sp, text};

/// One group: its heading and `(label, keys)` rows.
pub type ShortcutGroup = (&'static str, Vec<(String, Vec<Kbd>)>);

fn key_cap(key: Kbd, cx: &App) -> Kbd {
    let size = sp(crate::design::tokens().kbd.size);
    Styled::h(key, size)
        .min_w(size)
        .flex()
        .items_center()
        .justify_center()
        .px(gap::tight())
        .rounded(radius::base())
        .bg(color::muted(cx))
        .text_color(color::muted_foreground(cx))
        .text_size(text::meta())
        .font_weight(FontWeight::MEDIUM)
}

/// Scrolling list of groups, label left and key caps right.
pub fn shortcut_list(groups: Vec<ShortcutGroup>, cx: &App) -> Stateful<Div> {
    v_flex()
        .id("shortcuts")
        .max_h(sp(130.))
        .overflow_y_scroll()
        .gap(gap::group())
        .children(groups.into_iter().map(|(group, rows)| {
            v_flex()
                .gap(gap::item())
                .child(subheading(group).text_color(color::muted_foreground(cx)))
                .children(rows.into_iter().map(|(label, keys)| {
                    h_flex()
                        .justify_between()
                        .text_size(text::body())
                        .child(label)
                        .child(
                            h_flex()
                                .gap(gap::tight())
                                .children(keys.into_iter().map(|key| key_cap(key, cx))),
                        )
                }))
        }))
}
