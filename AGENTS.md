### UI

Built with [gpui-kit](https://gpui-kit.com/component/) (components) on top of [GPUI](https://gpui.rs/) (the underlying Rust UI framework, docs + examples at `crates/gpui/examples/*` in Zed's repo).

- Use gpui-kit's components instead of hand-rolling raw `div()` trees where one already fits.
- Always pull colors from the active theme (`cx.theme()...`); never hardcode a color.
- Create reusable components as necessary rather than duplicating a layout.
- To round an image's own corners, call `.rounded()`/`.rounded_full()` directly on the `img()` element itself, not only on a wrapping `div`. A wrapper's `overflow_hidden()` only clips to its own rectangle; the image only picks up rounded corners when it carries its own `rounded()`. This is Zed's own production pattern (`ui::Avatar`: both the wrapping `div` and the inner `img()` get `.rounded_full()`).

### Verifying a UI change before trusting what you see

- `cargo build --release` (and `cargo check`) can finish instantly and report success without recompiling if it thinks nothing changed — this can mask a real edit not landing in the binary you're about to screenshot.
- Before treating a screenshot as evidence for or against a code change, confirm the build actually recompiled: look for `Compiling tbis` in the build output, not just `Finished`. If it's missing, `touch` the changed files and rebuild.
- Don't build a theory about the framework (e.g. "the renderer doesn't support X") from a visual diff until a fresh, confirmed-recompiled build has been screenshotted. A stale binary looks identical to a real regression.
