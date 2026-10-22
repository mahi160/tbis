//! Bounded image cache for long poster grids: keeps the most recently drawn images
//! and frees the least recently drawn beyond a fixed count.

use std::collections::HashMap;
use std::sync::Arc;

use gpui_kit::*;

/// `image_cache(...)` provider holding at most `max` images, kept in element state under `id`.
pub fn lru_images(id: impl Into<ElementId>, max: usize) -> LruProvider {
    LruProvider { id: id.into(), max }
}

pub struct LruProvider {
    id: ElementId,
    max: usize,
}

impl ImageCacheProvider for LruProvider {
    fn provide(&mut self, window: &mut Window, cx: &mut App) -> AnyImageCache {
        let max = self.max;
        window
            .with_global_id(self.id.clone(), |id, window| {
                window.with_element_state::<Entity<LruImages>, _>(id, |cache, _| {
                    let cache = cache.unwrap_or_else(|| cx.new(|cx| LruImages::new(max, cx)));
                    (cache.clone(), cache)
                })
            })
            .into()
    }
}

pub struct LruImages {
    max: usize,
    /// Bumped on every load; an entry's stamp is when it was last drawn.
    clock: u64,
    images: HashMap<u64, (ImageCacheItem, u64)>,
}

impl LruImages {
    fn new(max: usize, cx: &mut Context<Self>) -> Self {
        cx.on_release(|this, cx| {
            for (item, _) in std::mem::take(&mut this.images).into_values() {
                if let Some(Ok(image)) = item.get() {
                    cx.drop_image(image, None);
                }
            }
        })
        .detach();
        Self {
            max,
            clock: 0,
            images: HashMap::new(),
        }
    }
}

impl ImageCache for LruImages {
    fn load(
        &mut self,
        resource: &Resource,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        self.clock += 1;
        let key = hash(resource);
        if let Some((item, used)) = self.images.get_mut(&key) {
            *used = self.clock;
            return item.use_image(window);
        }
        if self.images.len() >= self.max
            && let Some(oldest) = self
                .images
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(key, _)| *key)
            && let Some((item, _)) = self.images.remove(&oldest)
            && let Some(Ok(image)) = item.get()
        {
            cx.drop_image(image, Some(window));
        }
        let item = ImageCacheItem::new(resource, cx);
        let result = item.use_image(window);
        self.images.insert(key, (item, self.clock));
        result
    }
}
