use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::jellyfin::{Api, Item};

/// Emitted by any screen when the user picks a Movie or Episode to play.
pub struct Play(pub Item);

/// Emitted when the user picks a Series; opens Series detail.
pub struct OpenSeries(pub Item);

const TITLE_HEIGHT: f32 = 20.;
const META_HEIGHT: f32 = 16.;

pub type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// 2:3 poster, title, year; check badge when played. Fixed height so grid rows stay uniform.
pub fn poster_card(
    api: &Api,
    item: &Item,
    width: Pixels,
    on_click: Option<OnClick>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let name: SharedString = item.name.clone().into();
    v_flex()
        .id(SharedString::from(item.id.clone()))
        .w(width)
        .gap_1()
        .when_some(on_click, |this, on_click| {
            this.cursor_pointer().on_click(on_click)
        })
        .child(
            div()
                .relative()
                .w(width)
                .h(width * 1.5)
                .rounded_md()
                .overflow_hidden()
                .bg(theme.muted)
                .child(image(api.poster_url(item), name.clone(), cx))
                .when(item.user_data.played, |this| this.child(check_badge(cx))),
        )
        .child(div().h(px(TITLE_HEIGHT)).text_sm().truncate().child(name))
        .child(
            div()
                .h(px(META_HEIGHT))
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(
                    item.production_year
                        .map(|y| y.to_string())
                        .unwrap_or_default(),
                ),
        )
        .into_any_element()
}

/// 16:9 art, title, `S2E3 · Name` or year, progress bar. For Continue Watching and Next Up.
pub fn wide_card(api: &Api, item: &Item, width: Pixels, on_click: OnClick, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let (title, subtitle) = match &item.series_name {
        Some(series) => (series.clone(), item.episode_label()),
        None => (
            item.name.clone(),
            item.production_year
                .map(|y| y.to_string())
                .unwrap_or_default(),
        ),
    };
    v_flex()
        .id(SharedString::from(item.id.clone()))
        .w(width)
        .flex_shrink_0()
        .gap_1()
        .cursor_pointer()
        .on_click(on_click)
        .child(
            div()
                .relative()
                .w(width)
                .h(width * (9. / 16.))
                .rounded_md()
                .overflow_hidden()
                .bg(theme.muted)
                .child(image(api.wide_image_url(item), title.clone().into(), cx))
                .when_some(
                    item.user_data.played_percentage.filter(|p| *p > 0.),
                    |this, p| this.child(progress_bar(p, cx)),
                ),
        )
        .child(div().text_sm().truncate().child(title))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .truncate()
                .child(subtitle),
        )
        .into_any_element()
}

/// Round check in the top-right corner of a relative image box.
pub fn check_badge(cx: &App) -> Div {
    div()
        .absolute()
        .top_2()
        .right_2()
        .size_6()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(cx.theme().primary)
        .text_color(cx.theme().primary_foreground)
        .child(Icon::new(IconName::Check).small())
}

/// Thin bar along the bottom of a relative image box; `percent` is 0..100.
pub fn progress_bar(percent: f64, cx: &App) -> Div {
    div()
        .absolute()
        .bottom_0()
        .left_0()
        .right_0()
        .h_1()
        .bg(hsla(0., 0., 0., 0.5))
        .child(
            div()
                .h_full()
                .w(relative((percent / 100.) as f32))
                .bg(cx.theme().primary),
        )
}

/// Image filling its parent; `fallback_text` on a muted tile when missing.
pub fn image(url: Option<String>, fallback_text: SharedString, cx: &App) -> AnyElement {
    let (muted, muted_fg) = (cx.theme().muted, cx.theme().muted_foreground);
    let placeholder = move |text: SharedString| {
        div()
            .size_full()
            .bg(muted)
            .flex()
            .items_center()
            .justify_center()
            .p_2()
            .text_sm()
            .text_center()
            .text_color(muted_fg)
            .child(text)
            .into_any_element()
    };
    match url {
        Some(url) => img(url)
            .size_full()
            .object_fit(ObjectFit::Cover)
            .with_fallback(move || placeholder(fallback_text.clone()))
            .into_any_element(),
        None => placeholder(fallback_text),
    }
}
