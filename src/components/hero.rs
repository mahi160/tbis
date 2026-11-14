//! Featured films at the top of the Modern view: a carousel of wide slides (art, logo or
//! title, one meta line, two-line overview, Play / More info) with dots to pick one.

use std::time::Duration;

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{IconName, h_flex, v_flex};
use gpui_kit::*;

use super::art;
use super::button::{VegaSize as _, glass_button, primary_button};
use super::tag::{overlay_pill, star_rating};
use super::{OnClick, OnHover};
use crate::design::overlay::{self, black};
use crate::design::size::HERO_SLIDE_W;
use crate::design::{color, gap, page, radius, size, text};

const FADE: Duration = Duration::from_millis(600);

/// A full-size layer of the card. Every layer that paints carries the card's radius:
/// the card's `overflow_hidden` clips to its rectangle, not its rounded corners.
fn layer() -> Div {
    div().absolute().inset_0().rounded(radius::base())
}

/// One slide; `logo` falls back to `title` when missing or broken.
pub struct Hero {
    pub backdrop: Option<String>,
    pub logo: Option<String>,
    pub title: SharedString,
    /// Plain facts in reading order, e.g. year and runtime.
    pub facts: Vec<SharedString>,
    pub rating: Option<f32>,
    /// Age rating and format badges (4K, HDR, Atmos).
    pub tags: Vec<SharedString>,
    pub overview: Option<SharedString>,
    pub on_play: OnClick,
    pub on_more: OnClick,
}

impl Hero {
    /// Fills its `relative` parent; `ix` keeps the buttons' ids apart while two slides
    /// overlap mid-fade.
    fn render(self, ix: usize, cx: &App) -> Div {
        let name = self.title.clone();
        let fallback = move || {
            div()
                .text_size(text::display())
                .font_weight(FontWeight::BOLD)
                .line_clamp(2)
                .child(name.clone())
                .into_any_element()
        };
        let heading = match self.logo {
            Some(url) => img(url)
                .h(size::hero_logo_h())
                .max_w(size::hero_logo_max_w())
                .object_fit(ObjectFit::Contain)
                .with_fallback(fallback)
                .into_any_element(),
            None => fallback(),
        };
        let meta = h_flex()
            .gap(gap::item())
            .items_center()
            .flex_wrap()
            .text_size(text::lead())
            .font_weight(FontWeight::MEDIUM)
            .children(self.facts)
            .children(self.rating.map(|score| star_rating(score, cx)))
            .children(self.tags.into_iter().map(overlay_pill));
        layer()
            .child(art::image(self.backdrop, cx))
            // scrims: the text column sits bottom-left on any image or theme
            .child(layer().bg(linear_gradient(
                90.,
                linear_color_stop(black(0.7), 0.),
                linear_color_stop(overlay::CLEAR, 0.6),
            )))
            .child(layer().bg(linear_gradient(
                0.,
                linear_color_stop(black(0.5), 0.),
                linear_color_stop(overlay::CLEAR, 0.5),
            )))
            .child(
                v_flex()
                    .absolute()
                    .left_0()
                    .bottom_0()
                    .w_full()
                    .max_w(size::hero_text_w())
                    .p(page())
                    .gap(gap::group())
                    .items_start()
                    .text_color(overlay::WHITE)
                    .child(heading)
                    .child(meta)
                    .children(self.overview.map(|overview| {
                        div()
                            .text_size(text::lead())
                            .text_color(overlay::TEXT_MUTED)
                            .line_clamp(2)
                            .text_ellipsis()
                            .child(overview)
                    }))
                    .child(
                        h_flex()
                            .gap(gap::item())
                            .child(
                                primary_button(("hero-play", ix), "Play")
                                    .icon(IconName::Play)
                                    .xl()
                                    .on_click(self.on_play),
                            )
                            .child(
                                glass_button(("hero-more", ix), "More info")
                                    .xl()
                                    .on_click(self.on_more),
                            ),
                    ),
            )
    }
}

/// A neighbouring slide peeking in at the side: dimmed art only; clicking shows it.
pub struct Peek {
    pub backdrop: Option<String>,
    pub title: SharedString,
    pub on_click: OnClick,
}

impl Peek {
    fn render(self, side: usize, cx: &App) -> Stateful<Div> {
        let title = self.title;
        div()
            .id(("hero-peek", side))
            .group("hero-peek")
            .relative()
            .size_full()
            .rounded(radius::base())
            .overflow_hidden()
            .cursor_pointer()
            .tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
            .on_click(self.on_click)
            .child(art::image(self.backdrop, cx))
            .child(
                layer()
                    .bg(black(0.55))
                    .group_hover("hero-peek", |this| this.bg(black(0.35))),
            )
    }
}

/// The hero carousel: the current slide centered, its neighbours peeking in dimmed
/// at both edges. `slide` fades in over `previous` once per `generation`. One dot
/// per slide; the active one is a pill that fills over `dwell`, restarting per `cycle`,
/// and stays full while paused. The slide takes a share of the width up to a cap.
#[derive(IntoElement)]
pub struct HeroCarousel {
    pub slide: Hero,
    pub previous: Option<Hero>,
    /// The slides before and after; `None` with a single slide.
    pub peeks: Option<[Peek; 2]>,
    pub active: usize,
    pub generation: usize,
    /// Keys the active dot's fill; bumped whenever the countdown restarts.
    pub cycle: usize,
    /// Time until the next slide; `None` while paused.
    pub dwell: Option<Duration>,
    /// Per slide: its title (the dot's tooltip) and what clicking the dot does.
    pub dots: Vec<(SharedString, OnClick)>,
    /// Pointer entered or left the slide; auto-advance pauses meanwhile.
    pub on_hover: OnHover,
}

impl RenderOnce for HeroCarousel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let active = self.active;
        let slide = self.slide.render(active, cx);
        let slide = if self.previous.is_none() || cx.reduce_motion() {
            slide.into_any_element()
        } else {
            slide
                .with_animation(
                    ElementId::NamedInteger("hero-fade".into(), self.generation as u64),
                    Animation::new(FADE).with_easing(ease_in_out),
                    |slide, delta| slide.opacity(delta),
                )
                .into_any_element()
        };
        let previous = self
            .previous
            .map(|hero| hero.render(usize::MAX, cx).into_any_element());
        let card = div()
            .id("hero")
            .relative()
            .rounded(radius::base())
            .overflow_hidden()
            .on_hover(self.on_hover)
            .children(previous)
            .child(slide);
        let (lit, unlit) = (
            color::foreground(cx),
            color::muted_foreground(cx).opacity(0.5),
        );
        let dwell = self.dwell.filter(|_| !cx.reduce_motion());
        let cycle = self.cycle;
        let fill = move || -> AnyElement {
            let bar = div().h_full().rounded(radius::base()).bg(lit);
            match dwell {
                Some(dwell) => bar
                    .with_animation(
                        ElementId::NamedInteger("hero-countdown".into(), cycle as u64),
                        Animation::new(dwell),
                        |bar, delta| bar.w(relative(delta)),
                    )
                    .into_any_element(),
                None => bar.w_full().into_any_element(),
            }
        };
        let dots =
            (self.dots.len() > 1).then(|| {
                h_flex().children(self.dots.into_iter().enumerate().map(
                    |(ix, (title, on_click))| {
                        let on = ix == active;
                        let dot = div()
                            .h(size::dot())
                            .rounded(radius::base())
                            .bg(unlit)
                            .overflow_hidden();
                        let dot = if on {
                            dot.w(size::dot_active_w()).child(fill())
                        } else {
                            dot.w(size::dot())
                        };
                        // padded hit area around the small dot
                        div()
                            .id(("hero-dot", ix))
                            .p(gap::tight())
                            .cursor_pointer()
                            .tooltip(move |window, cx| {
                                Tooltip::new(title.clone()).build(window, cx)
                            })
                            .on_click(on_click)
                            .child(dot)
                    },
                ))
            });
        // the cap sits on a wrapper: on the card itself, layout sizes the height
        // from the uncapped width
        let slot = |child: AnyElement| {
            div()
                .flex_shrink_0()
                .w(relative(HERO_SLIDE_W))
                .max_w(size::hero_max_w())
                .child(
                    div()
                        .w_full()
                        .min_h(size::hero_min_h())
                        .aspect_ratio(size::HERO_ASPECT)
                        .child(child),
                )
        };
        let [before, after] = match self.peeks {
            Some([before, after]) => [
                Some(slot(before.render(0, cx).into_any_element())),
                Some(slot(after.render(1, cx).into_any_element())),
            ],
            None => [None, None],
        };
        v_flex()
            .w_full()
            .items_center()
            .gap(gap::item())
            .child(
                // centered and wider than the window: the peeks overflow both edges
                h_flex()
                    .w_full()
                    .justify_center()
                    .gap(gap::item())
                    .overflow_hidden()
                    .children(before)
                    .child(slot(card.size_full().into_any_element()))
                    .children(after),
            )
            .children(dots)
    }
}
