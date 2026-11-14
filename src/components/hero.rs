//! Featured films at the top of the Modern view: a carousel of wide slides (art, logo or
//! title, one meta line, two-line overview, Play / More info). The neighbouring films
//! peek in smaller and dimmed at both edges; changing slide slides the strip.

use std::time::Duration;

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{IconName, h_flex, v_flex};
use gpui_kit::*;

use super::art;
use super::button::{VegaSize as _, glass_button, primary_button};
use super::tag::{overlay_pill, star_rating};
use super::{OnClick, OnHover};
use crate::design::overlay::{self, black};
use crate::design::size::{HERO_ASPECT, HERO_PEEK_SCALE, HERO_SLIDE_W};
use crate::design::{color, gap, page, radius, size, text};

const SLIDE: Duration = Duration::from_millis(600);
/// How dark a peeking slide's art is; it brightens on the way to the center.
const PEEK_DIM: f32 = 0.55;
/// Offsets from the current slide of the films drawn either side; the outer two are
/// off screen until a slide brings them in.
pub const SIDES: [i32; 4] = [-2, -1, 1, 2];

/// A full-size layer of the card. Every layer that paints carries the card's radius:
/// the card's `overflow_hidden` clips to its rectangle, not its rounded corners.
fn layer() -> Div {
    div().absolute().inset_0().rounded(radius::base())
}

/// The current slide's text and buttons; `logo` falls back to `title` when missing
/// or broken.
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
    /// Scrims and the bottom-left text column, over the slide's art.
    fn content(self, cx: &App) -> Div {
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
                                primary_button("hero-play", "Play")
                                    .icon(IconName::Play)
                                    .xl()
                                    .on_click(self.on_play),
                            )
                            .child(
                                glass_button("hero-more", "More info")
                                    .xl()
                                    .on_click(self.on_more),
                            ),
                    ),
            )
    }
}

/// A neighbouring film: dimmed art only; clicking shows it.
pub struct Peek {
    pub backdrop: Option<String>,
    pub title: SharedString,
    pub on_click: OnClick,
}

/// Where slides sit for the current window, in pixels: the current slide centered,
/// smaller peeks either side.
#[derive(Clone, Copy)]
struct Strip {
    width: f32,
    card_w: f32,
    card_h: f32,
    gap: f32,
}

impl Strip {
    fn new(window: &Window) -> Self {
        let width = window.viewport_size().width.as_f32();
        let card_w = (width * HERO_SLIDE_W).min(size::hero_max_w().as_f32());
        Self {
            width,
            card_w,
            card_h: (card_w / HERO_ASPECT).max(size::hero_min_h().as_f32()),
            gap: gap::item().as_f32(),
        }
    }

    /// Places a slide at `at` slots from the center (fractional mid-slide): full size
    /// at 0, peek size from 1 out, vertically centered on the current slide.
    fn place<E: Styled>(self, slide: E, at: f32) -> E {
        let near = at.abs().min(1.);
        let scale = 1. - (1. - HERO_PEEK_SCALE) * near;
        let (peek_w, w, h) = (
            self.card_w * HERO_PEEK_SCALE,
            self.card_w * scale,
            self.card_h * scale,
        );
        let reach = near * (self.card_w + peek_w) / 2.
            + near * self.gap
            + (at.abs() - 1.).max(0.) * (peek_w + self.gap);
        let center = self.width / 2. + at.signum() * reach;
        slide
            .left(px(center - w / 2.))
            .top(px((self.card_h - h) / 2.))
            .w(px(w))
            .h(px(h))
    }
}

/// The hero carousel. One dot per slide; the active one is a pill that fills over
/// `dwell`, restarting per `cycle`, and stays full while paused.
#[derive(IntoElement)]
pub struct HeroCarousel {
    pub slide: Hero,
    /// The films at `SIDES` offsets; `None` with a single slide.
    pub sides: Option<[Peek; 4]>,
    /// The way the strip moved to this slide (1 forward, -1 back), once per
    /// `generation`; 0 shows it in place.
    pub direction: i32,
    pub active: usize,
    pub generation: usize,
    /// Keys the active dot's fill; bumped whenever the countdown restarts.
    pub cycle: usize,
    /// Time until the next slide; `None` while paused.
    pub dwell: Option<Duration>,
    /// Per slide: its title (the dot's tooltip) and what clicking the dot does.
    pub dots: Vec<(SharedString, OnClick)>,
    /// Pointer entered or left the current slide; auto-advance pauses meanwhile.
    pub on_hover: OnHover,
}

impl RenderOnce for HeroCarousel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let strip = Strip::new(window);
        let direction = if cx.reduce_motion() {
            0
        } else {
            self.direction
        };
        let generation = self.generation as u64;
        // `at` slots from the center at `t` of the slide (0 to 1)
        let at = move |offset: i32, t: f32| offset as f32 + direction as f32 * (1. - t);
        // positions a slide (and dims its art) per frame while the strip moves
        let animate = move |offset: i32, slide: Stateful<Div>, dim: Div| {
            let shade =
                move |dim: Div, t: f32| dim.bg(black(PEEK_DIM * at(offset, t).abs().min(1.)));
            if direction == 0 {
                return strip
                    .place(slide, at(offset, 1.))
                    .child(shade(dim, 1.))
                    .into_any_element();
            }
            let id = |part: &str| {
                ElementId::NamedInteger(format!("hero-{part}-{offset}").into(), generation)
            };
            let motion = || Animation::new(SLIDE).with_easing(ease_in_out);
            slide
                .child(dim.with_animation(id("dim"), motion(), shade))
                .with_animation(id("slide"), motion(), move |slide, t| {
                    strip.place(slide, at(offset, t))
                })
                .into_any_element()
        };
        let card = |offset: i32, backdrop: Option<String>, cx: &App| {
            div()
                .id(("hero-slide", (offset + 2) as usize))
                .absolute()
                .rounded(radius::base())
                .overflow_hidden()
                .child(art::image(backdrop, cx))
        };

        let backdrop = self.slide.backdrop.clone();
        let content = self.slide.content(cx);
        let content = if direction == 0 {
            content.into_any_element()
        } else {
            content
                .with_animation(
                    ElementId::NamedInteger("hero-content".into(), generation),
                    // text lands late, once the slide has mostly arrived
                    Animation::new(SLIDE).with_easing(quadratic),
                    |content, t| content.opacity(t),
                )
                .into_any_element()
        };
        let current = card(0, backdrop, cx).on_hover(self.on_hover).child(content);
        let current = animate(0, current, layer());
        let sides = self
            .sides
            .into_iter()
            .flatten()
            .zip(SIDES)
            .map(|(peek, offset)| {
                let title = peek.title;
                let slide = card(offset, peek.backdrop, cx)
                    .group("hero-peek")
                    .cursor_pointer()
                    .tooltip(move |window, cx| Tooltip::new(title.clone()).build(window, cx))
                    .on_click(peek.on_click);
                let dim = layer().group_hover("hero-peek", |this| this.bg(black(PEEK_DIM / 2.)));
                animate(offset, slide, dim)
            });

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
        let active = self.active;
        let dots =
            (self.dots.len() > 1).then(|| {
                h_flex().children(self.dots.into_iter().enumerate().map(
                    |(ix, (title, on_click))| {
                        let dot = div()
                            .h(size::dot())
                            .rounded(radius::base())
                            .bg(unlit)
                            .overflow_hidden();
                        let dot = if ix == active {
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

        v_flex()
            .w_full()
            .items_center()
            .gap(gap::item())
            .child(
                // the current slide last, so it paints over a peek passing under it
                div()
                    .relative()
                    .w_full()
                    .h(px(strip.card_h))
                    .overflow_hidden()
                    .children(sides)
                    .child(current),
            )
            .children(dots)
    }
}
