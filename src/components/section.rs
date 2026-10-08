//! Titled page section: semibold heading over its body.

use gpui_kit::component::v_flex;
use gpui_kit::*;

pub fn section(title: impl Into<SharedString>, body: impl IntoElement) -> AnyElement {
    v_flex()
        .gap(crate::design::gap::group())
        .child(super::typography::section_heading(title.into()))
        .child(body)
        .into_any_element()
}
