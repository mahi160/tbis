# 30 — Hero component

**What to build:** A hero for the top of the Modern view: a wide backdrop with a gradient, the item's logo (or its title when it has no logo), one line of meta, a two-line overview, and Play and More info buttons. A stateless component only; nothing uses it until ticket 31.

**Model:** sonnet

**Blocked by:** 28 (needs `logo_url`).

**Status:** done. `Hero` is a `RenderOnce` struct with public fields; a seven-argument builder would trip Clippy's `too_many_arguments`. `overview` is optional because `Item::overview` is. When the logo image fails to load, the title shows instead. `size::hero_h` and `size::hero_logo_h` sit outside `units!` until ticket 31 uses them, so they can carry `#[allow(dead_code)]`. Placed temporarily on Home and screenshotted in three cases (logo, long title without a logo, no backdrop), then removed.

**Read first:** `AGENTS.md` (every UI rule; the image-rounding rule matters here), `src/components/backdrop.rs` (top-of-page art with a gradient: the base to build on), `src/components/detail.rs` (`detail_header`, `meta_line`), `src/components/button.rs`, `src/components/typography.rs`, `src/design.rs` (`size`, `gap`, `text`, `overlay`).

## Steps

1. Create `src/components/hero.rs` and register it in `src/components/mod.rs`.
2. Add a size to `design::size` for the hero height (a multiple of the spacing unit, as the other sizes are); do not write a pixel literal in the component.
3. Build a `RenderOnce` component or builder function taking: backdrop URL, optional logo URL, title, meta text, overview, and two click handlers (`on_play`, `on_more`).
4. Layout: backdrop fills the hero; a left-to-right dark gradient from `design::overlay` keeps text readable on any image; the text column sits at the bottom left with `page()` padding, in this order: logo image (fixed height, `ObjectFit::Contain`) or the title in `text::title()`, the meta line, the overview limited to two lines, then the buttons from `components/button.rs` (primary Play, outline More info).
5. Text over the image uses the `overlay` whites, as the Player's chrome does, so it reads on every theme.
6. Handle a missing backdrop with the same placeholder treatment `components/art.rs` uses.

## Done when

- [x] `hero.rs` builds only from `components/` and `design` tokens: no `px(...)` literal beyond 0 or 1, no hardcoded colour, no `Button::new`
- [x] It renders with a logo, without a logo, and without a backdrop
- [x] Long titles and overviews truncate instead of overflowing

**Check:** the gate in `PLAN.md`. Until ticket 31 uses it, mark the public function `#[allow(dead_code)]` with the comment `// used by the Modern view (roadmap 31)`. To look at it, temporarily place it at the top of Home, take a screenshot, and remove it again.
