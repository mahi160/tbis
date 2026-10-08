### UI

Built with [gpui-kit](https://gpui-kit.com/component/) (components) on top of [GPUI](https://gpui.rs/) (the underlying Rust UI framework, docs + examples at `crates/gpui/examples/*` in Zed's repo).

- All UI pieces live in `src/components/`. Pages and features compose components from there; they never hand-build UI inline.
- Build each component on a gpui-kit component when one fits. Write custom element code only inside `src/components/`, and only when gpui-kit has no fitting component.
- Every stateless piece (`RenderOnce` or a builder function) belongs in `src/components/`. Reusable stateful widgets (an `Entity` with `Render` that holds only UI state, not app or API state) belong there too.
- Before adding UI to a page, check `src/components/` for an existing component and reuse or extend it.
- Create every button, text or icon-only, through `components/button.rs`; set size with its `VegaSize` methods (`.xs()`, `.sm()`, `.lg()`, `.icon_xs()`), never gpui-kit's `.small()` / `.large()`. Never call `Button::new` outside `components/button.rs`, so a single edit restyles every button.
- `src/design.rs` is the single source UI code reads for every visual value. It is the only file allowed to call `cx.theme()`; components and pages use `design::color` and `design::font` instead, and never hardcode a color.
- Design tokens live in `src/design.rs`: `color` and `font` (from the theme), `text`, `radius`, `gap`, `size` (sizes shared across files), `window`, `opacity`, `overlay` (`black(a)` / `white(a)` tints over video and images), and `shadow`.
- The system is deliberately small, the same for every theme. Don't add steps:
  - Type: three sizes only, `text::meta()` (details), `text::body()` (everything else), `text::title()` (headings). Build hierarchy with weight and muted color.
  - Radius: one, `radius::base()`, for everything.
  - Spacing between and inside things: three gaps only, `gap::tight()`, `gap::item()`, `gap::group()`, plus `page()` for the window padding around every page.
  - Color: a theme sets 15 roles (`theme::ROLES`: background, foreground, surface, muted bg/fg, border, subtle border, hover, active, selection, primary bg/fg, focus, danger bg/fg); `theme::apply` derives every other gpui-kit color from them.
- Fixed dimensions (control heights, poster widths, offsets that clear chrome) are multiples of the theme's spacing unit: `sp(n)`. Never use `sp(n)` for a gap or padding. Never write `px(...)` literals in UI code, except 0px/1px layout and rounding. When a page or a second file needs a size, name it in `design::size`.
- Avoid gpui's numbered shorthands such as `.gap_2()` or `.text_sm()`; use `.gap(gap::item())` and `.text_size(text::body())`.
- One theme entry in `assets/themes/tbis.json` is the whole look: the 15 color roles, font families, and a `"design"` object (`design::Design`: spacing unit, `text`, `radius`, `gap`, shadow strength, component heights). Missing keys fall back to defaults; unknown keys and non-role colors fail the theme tests. Don't set `font.size`, `radius` or `radius.lg` at the theme's top level: `theme::apply` derives them from `design`.
- Create reusable components as necessary rather than duplicating a layout.
- To round an image's own corners, call `.rounded()`/`.rounded_full()` directly on the `img()` element itself, not only on a wrapping `div`. A wrapper's `overflow_hidden()` only clips to its own rectangle; the image only picks up rounded corners when it carries its own `rounded()`. This is Zed's own production pattern (`ui::Avatar`: both the wrapping `div` and the inner `img()` get `.rounded_full()`).

### Verifying a UI change before trusting what you see

- `cargo build --release` (and `cargo check`) can finish instantly and report success without recompiling if it thinks nothing changed — this can mask a real edit not landing in the binary you're about to screenshot.
- Before treating a screenshot as evidence for or against a code change, confirm the build actually recompiled: look for `Compiling tbis` in the build output, not just `Finished`. If it's missing, `touch` the changed files and rebuild.
- Don't build a theory about the framework (e.g. "the renderer doesn't support X") from a visual diff until a fresh, confirmed-recompiled build has been screenshotted. A stale binary looks identical to a real regression.
