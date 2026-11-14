# 20 — Bounded image memory on Home and Search

**What to build:** The Movies and Series grids keep at most 200 decoded posters each (`lru_images`). Home rows and Search results have no bound, so their decoded images stay in memory. Give them the same bounded cache.

**Model:** haiku

**Blocked by:** None.

**Status:** ready-for-agent

**Read first:** `src/components/image_cache.rs` (all), `src/library.rs` (`fn render`, the `image_cache(lru_images(...))` wrapper and `POSTERS_KEPT`), `src/home.rs` (`fn render`), `src/search.rs` (`fn render`).

## Steps

1. In `src/home.rs`, add `const IMAGES_KEPT: usize = 120;` with the comment `// four rows of up to 24 cards, with room to scroll back`.
2. In `fn render`, wrap the element that holds the rows (the `rows_page("home")` tree) in `image_cache(lru_images("home-images", IMAGES_KEPT))`, the way `library.rs` wraps its grid. The wrapper is a container: move the layout calls that size the page (`flex_1`, `min_h_0`, `size_full`) onto the wrapper as `library.rs` does, so the page still fills its space.
3. Do the same in `src/search.rs` with `"search-images"` and a constant of `120`.
4. Import `image_cache` and `lru_images` the way `library.rs` does.

## Done when

- [ ] Home and Search are wrapped in an `lru_images` cache
- [ ] Both pages look and scroll exactly as before
- [ ] Scrolling back to an evicted card reloads its image

**Check:** the gate in `PLAN.md`, then open Home and Search and compare the layout with the current build. Follow the build-freshness rule in `AGENTS.md` before trusting what you see.
