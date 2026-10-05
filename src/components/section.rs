//! Titled page section: bold heading over its body.

use gpui_kit::component::v_flex;
use gpui_kit::*;

pub fn section(title: impl Into<SharedString>, body: impl IntoElement) -> AnyElement {
    v_flex()
        .gap(crate::design::sp(3.))
        .child(super::typography::title(title.into()))
        .child(body)
        .into_any_element()
}
