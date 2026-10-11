//! Library backdrops behind Home, Movies and Series (Settings > Appearance > Backdrops),
//! as in Jellyfin web: a random item's backdrop, dimmed, cross-fading to another every
//! `ROTATE`. Movies and Series pick from their own kind; Home from both.

use std::time::Duration;

use gpui_kit::*;

use crate::components::page_backdrop::page_backdrop;
use crate::design::backdrop::BLUR;
use crate::jellyfin::{Api, Kind};

const ROTATE: Duration = Duration::from_secs(30);
/// Backdrops fetched per pool; cycling them all takes `POOL * ROTATE`.
const POOL: usize = 30;

/// Pool index: Home (`None`, Movies and Series), Movies, Series.
fn slot(kind: Option<Kind>) -> usize {
    match kind {
        Some(Kind::Movie) => 1,
        Some(Kind::Series) => 2,
        _ => 0,
    }
}

pub struct Backdrops {
    api: Api,
    /// Backdrop URLs per pool in the server's random order; `None` until loaded.
    pools: [Option<Vec<String>>; 3],
    slot: usize,
    index: usize,
    current: Option<String>,
    /// Fading out under `current`.
    previous: Option<String>,
    /// Re-keys the fade per swap.
    generation: u64,
    _load: Task<()>,
    _rotate: Task<()>,
}

impl Backdrops {
    pub fn new(api: Api, cx: &mut Context<Self>) -> Self {
        let _rotate = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(ROTATE).await;
                if this.update(cx, |this, cx| this.advance(cx)).is_err() {
                    break;
                }
            }
        });
        Self {
            api,
            pools: Default::default(),
            slot: 0,
            index: 0,
            current: None,
            previous: None,
            generation: 0,
            _load: Task::ready(()),
            _rotate,
        }
    }

    /// Switches to the pool for `kind` (`None`: Home), loading it on first use.
    pub fn show(&mut self, kind: Option<Kind>, cx: &mut Context<Self>) {
        let slot = slot(kind);
        if slot == self.slot && self.current.is_some() {
            return;
        }
        self.slot = slot;
        self.index = 0;
        if self.pools[slot].is_some() {
            self.swap(cx);
            return;
        }
        let api = self.api.clone();
        self._load = cx.spawn(async move |this, cx| {
            // failure leaves the pool unloaded, so the next visit retries
            let urls = match api.random_backdrops(kind, POOL).await {
                Ok(items) => Some(
                    items
                        .iter()
                        .filter_map(|i| api.backdrop_url(i))
                        .map(|url| format!("{url}&blur={BLUR}"))
                        .collect(),
                ),
                Err(err) => {
                    eprintln!("backdrops failed to load: {err}");
                    None
                }
            };
            this.update(cx, |this, cx| {
                if urls.is_some() {
                    this.pools[slot] = urls;
                }
                // unloaded pool is empty: swap drops the previous tab's backdrop
                if this.slot == slot {
                    this.swap(cx);
                }
            })
            .ok();
        });
    }

    fn pool(&self) -> &[String] {
        self.pools[self.slot].as_deref().unwrap_or_default()
    }

    /// Shows the pool's entry at `index`, fading from what was showing.
    fn swap(&mut self, cx: &mut Context<Self>) {
        let url = self.pool().get(self.index).cloned();
        if url == self.current {
            return;
        }
        self.previous = std::mem::replace(&mut self.current, url);
        self.generation += 1;
        cx.notify();
    }

    fn advance(&mut self, cx: &mut Context<Self>) {
        let len = self.pool().len();
        if len > 1 {
            self.index = (self.index + 1) % len;
            self.swap(cx);
        }
    }

    fn next(&self) -> Option<String> {
        let pool = self.pool();
        (pool.len() > 1).then(|| pool[(self.index + 1) % pool.len()].clone())
    }
}

impl Render for Backdrops {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        page_backdrop(
            self.previous.clone(),
            self.current.clone(),
            self.next(),
            self.generation,
            cx,
        )
    }
}
