//! Featured item banner at the top of Home: backdrop, scrim, title, meta, actions.

use gpui_kit::component::{IconName, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::OnClick;
use super::art::{image, progress_bar};
use super::button::{button, primary_button};
use super::typography::title;
use crate::design::overlay::{self, black};
use crate::design::{gap, radius, size, text};
use crate::jellyfin::{Item, Kind};

/// Title line: an Episode shows its Series name.
fn heading(item: &Item) -> String {
    match item.kind {
        Kind::Episode => item.series_name.clone().unwrap_or_default(),
        _ => item.name.clone(),
    }
}

/// Episode code and name, else year and runtime.
fn meta(item: &Item) -> String {
    match item.kind {
        Kind::Episode => item.episode_label(),
        _ => [
            item.production_year.map(|y| y.to_string()),
            item.run_time_ticks.map(crate::detail::runtime_label),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · "),
    }
}

/// Full-width banner for `item` over its `backdrop`. Text sits on fixed overlay tints,
/// not theme colors: the art behind it is arbitrary. Resume when the item has a saved
/// position, else Play; Details opens the detail page.
pub fn hero(
    item: &Item,
    backdrop: Option<String>,
    on_play: OnClick,
    on_details: OnClick,
    cx: &App,
) -> AnyElement {
    let resumable = item.user_data.playback_position_ticks > 0;
    let scrim = |angle: f32, from: Hsla, to: Hsla, end: f32| {
        div()
            .absolute()
            .inset_0()
            .rounded(radius::base())
            .bg(linear_gradient(
                angle,
                linear_color_stop(from, 0.),
                linear_color_stop(to, end),
            ))
    };
    div()
        .relative()
        .w_full()
        .h(size::hero_h())
        .rounded(radius::base())
        .overflow_hidden()
        .child(image(backdrop, cx))
        // left: text sits there; bottom: keeps the progress bar visible
        .child(scrim(90., black(0.8), overlay::CLEAR, 0.7))
        .child(scrim(0., black(0.5), overlay::CLEAR, 0.4))
        .child(
            v_flex()
                .absolute()
                .inset_0()
                .justify_end()
                .p(gap::group())
                .child(
                    v_flex()
                        .max_w(size::prose_max_w())
                        .gap(gap::item())
                        .text_color(overlay::WHITE)
                        .child(title(heading(item)).truncate())
                        .child(
                            div()
                                .text_size(text::body())
                                .text_color(overlay::TEXT_MUTED)
                                .truncate()
                                .child(meta(item)),
                        )
                        .child(
                            h_flex()
                                .gap(gap::item())
                                .child(
                                    primary_button(
                                        "hero-play",
                                        if resumable { "Resume" } else { "Play" },
                                    )
                                    .icon(IconName::Play)
                                    .on_click(on_play),
                                )
                                .child(button("hero-details", "Details").on_click(on_details)),
                        ),
                ),
        )
        .when_some(
            item.user_data.played_percentage.filter(|p| *p > 0.),
            |this, p| {
                this.child(progress_bar(
                    ElementId::Name(format!("hero-progress-{}", item.id).into()),
                    p,
                    cx,
                ))
            },
        )
        .into_any_element()
}
