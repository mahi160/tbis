//! Ambient backdrop behind Home, Library and Search: the blurred art of the item
//! last under the pointer, cross-fading as the pointer moves between cards.
//! One app-wide entity, so cards set it on hover without re-rendering their page.

use std::time::Duration;

use gpui_kit::*;

use crate::design::{color, frost};

/// Pointer must rest this long on a card before the backdrop follows, so sweeping a row doesn't thrash.
const SETTLE: Duration = Duration::from_millis(150);

#[derive(Default)]
pub struct Ambient {
    current: Option<String>,
    /// Fading out underneath `current`.
    previous: Option<String>,
    /// Re-keys the fade animation per swap.
    generation: u64,
    /// Hovered, not yet settled.
    pending: Option<String>,
    _settle: Option<Task<()>>,
}

impl Ambient {
    fn show(&mut self, url: String, cx: &mut Context<Self>) {
        if self.current.as_ref() == Some(&url) {
            return;
        }
        self.previous = self.current.replace(url);
        self.generation += 1;
        cx.notify();
    }

    fn hover(&mut self, url: String, cx: &mut Context<Self>) {
        self.pending = Some(url);
        self._settle = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SETTLE).await;
            this.update(cx, |this, cx| {
                if let Some(url) = this.pending.take() {
                    this.show(url, cx);
                }
            })
            .ok();
        }));
    }

    fn seed(&mut self, url: String, cx: &mut Context<Self>) {
        if self.current.is_none() && self.pending.is_none() {
            self.show(url, cx);
        }
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.pending = None;
        self._settle = None;
        self.previous = None;
        self.current = None;
        self.generation += 1;
        cx.notify();
    }
}

fn fill(url: String) -> Img {
    img(url).absolute().size_full().object_fit(ObjectFit::Cover)
}

impl Render for Ambient {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = color::background(cx);
        let current = self.current.clone().map(|url| {
            if cx.reduce_motion() {
                return fill(url).into_any_element();
            }
            fill(url)
                .with_animation(
                    ElementId::NamedInteger("ambient-fade".into(), self.generation),
                    Animation::new(frost::AMBIENT_FADE),
                    |this, delta| this.opacity(delta),
                )
                .into_any_element()
        });
        div()
            .size_full()
            .overflow_hidden()
            .bg(bg)
            .children(self.previous.clone().map(fill))
            .children(current)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(bg.opacity(frost::AMBIENT_TINT)),
            )
    }
}

struct Handle(Entity<Ambient>);

impl Global for Handle {}

/// Creates the app-wide backdrop; call once at startup.
pub fn init(cx: &mut App) {
    let ambient = cx.new(|_| Ambient::default());
    cx.set_global(Handle(ambient));
}

/// The backdrop view, to place behind a page.
pub fn layer(cx: &App) -> Entity<Ambient> {
    cx.global::<Handle>().0.clone()
}

/// `on_hover` handler for a card: pointer entering follows with `url`; leaving keeps
/// the current backdrop. No `url` (no art) leaves it alone.
pub fn follow_hover(url: Option<String>) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
    move |hovered, _, cx| {
        if *hovered && let Some(url) = &url {
            let url = url.clone();
            layer(cx).update(cx, |ambient, cx| ambient.hover(url, cx));
        }
    }
}

/// Shows `url` unless something is already showing or about to.
pub fn seed(url: String, cx: &mut App) {
    layer(cx).update(cx, |ambient, cx| ambient.seed(url, cx));
}

pub fn clear(cx: &mut App) {
    layer(cx).update(cx, |ambient, cx| ambient.clear(cx));
}
