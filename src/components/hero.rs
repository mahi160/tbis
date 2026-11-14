//! Featured item at the top of the Modern view: wide art, logo or title, meta,
//! overview and the Play / More info actions.

use gpui_kit::component::{IconName, h_flex, v_flex};
use gpui_kit::*;

use super::OnClick;
use super::art;
use super::button::{outline_button, primary_button};
use super::typography::{paragraph, title};
use crate::design::overlay::{self, black};
use crate::design::{gap, page, radius, size, text};

/// Everything the hero shows; `logo` falls back to `title` when missing or broken.
/// Sits inside the page gutter, like the rows below it.
#[derive(IntoElement)]
pub struct Hero {
    pub backdrop: Option<String>,
    pub logo: Option<String>,
    pub title: SharedString,
    pub meta: SharedString,
    pub overview: Option<SharedString>,
    pub on_play: OnClick,
    pub on_more: OnClick,
}

impl RenderOnce for Hero {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let name = self.title.clone();
        let heading = match self.logo {
            Some(url) => img(url)
                .h(size::hero_logo_h())
                .max_w_full()
                .object_fit(ObjectFit::Contain)
                .with_fallback(move || title(name.clone()).truncate().into_any_element())
                .into_any_element(),
            None => title(name).truncate().into_any_element(),
        };
        let card = div()
            .relative()
            .w_full()
            .h(size::hero_h())
            .rounded(radius::base())
            .overflow_hidden()
            .child(art::image(self.backdrop, cx))
            // left scrim: the text column sits there on any image or theme
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(radius::base())
                    .bg(linear_gradient(
                        90.,
                        linear_color_stop(black(0.75), 0.),
                        linear_color_stop(overlay::CLEAR, 0.7),
                    )),
            )
            .child(
                v_flex()
                    .absolute()
                    .left_0()
                    .bottom_0()
                    .w_full()
                    .max_w(size::prose_max_w())
                    .p(page())
                    .gap(gap::item())
                    .items_start()
                    .text_color(overlay::WHITE)
                    .child(heading)
                    .child(
                        div()
                            .w_full()
                            .text_size(text::body())
                            .text_color(overlay::TEXT_MUTED)
                            .truncate()
                            .child(self.meta),
                    )
                    .children(self.overview.map(|o| paragraph(o, 2)))
                    .child(
                        h_flex()
                            .gap(gap::item())
                            .child(
                                primary_button("hero-play", "Play")
                                    .icon(IconName::Play)
                                    .on_click(self.on_play),
                            )
                            .child(
                                outline_button("hero-more", "More info", cx).on_click(self.on_more),
                            ),
                    ),
            );
        div().w_full().px(page()).child(card)
    }
}
