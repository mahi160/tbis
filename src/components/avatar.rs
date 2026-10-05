//! Round person image in shadcn vega's Avatar style: `rounded-full`, fallback initials
//! on `bg-muted text-muted-foreground`. Custom rather than gpui-kit Avatar, whose
//! fallback paints a per-name hue that can't be overridden.

use gpui_kit::*;

use crate::design::{color, sp, text};

/// Photo at `size`; initials of `name` while missing or failing to load.
/// Rounded on the `img` itself: a wrapper's `overflow_hidden` clips only to a rectangle.
pub fn avatar(url: Option<String>, name: impl Into<SharedString>, size: Pixels, cx: &App) -> AnyElement {
    let name: SharedString = name.into();
    let initials: SharedString = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
        .into();
    let (muted, muted_fg) = (color::muted(cx), color::muted_foreground(cx));
    let font = if size >= sp(8.) { text::sm() } else { text::xs() };
    let fallback = move || {
        div()
            .size(size)
            .flex_shrink_0()
            .rounded_full()
            .bg(muted)
            .flex()
            .items_center()
            .justify_center()
            .text_size(font)
            .text_color(muted_fg)
            .child(initials.clone())
            .into_any_element()
    };
    match url {
        Some(url) => img(url)
            .size(size)
            .flex_shrink_0()
            .rounded_full()
            .object_fit(ObjectFit::Cover)
            .with_fallback(fallback)
            .into_any_element(),
        None => fallback(),
    }
}
