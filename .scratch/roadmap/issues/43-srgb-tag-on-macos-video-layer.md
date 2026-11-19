# 43 — Tag the macOS video layer sRGB

**What to build:** `src/platform/macos/window.rs` tags gpui's Metal layer sRGB, because an untagged layer is sent to the display raw and wide-gamut (P3) screens oversaturate it. The video has its own Metal layer, and that one is not tagged. Tag it the same way, then have a person compare the picture.

**Model:** haiku for the code; a person with a P3 display (any recent MacBook) must do the check.

**Blocked by:** None.

**Status:** implemented; P3 acceptance pending. The UI and video layers share sRGB tagging. No P3 display comparison has been performed; do not infer visual correctness from compilation. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/platform/macos/window.rs` (`pub fn prepare` and its doc comment), `src/platform/macos/video.rs` (`Surface::attach`, where `CAMetalLayer::new()` is called).

## Steps

1. In `src/platform/macos/window.rs`, add above `pub fn prepare`:

   ```rust
   /// Tags `layer` sRGB so macOS color-matches it to the display.
   pub fn tag_srgb(layer: &CAMetalLayer) {
       let srgb = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
       layer.setColorspace(srgb.as_deref());
   }
   ```

   and replace the last two lines of `prepare` (`let srgb = ...;` and `layer.setColorspace(...);`) with `tag_srgb(&layer);`.

2. In `src/platform/macos/video.rs`, in `Surface::attach`, after `layer.setFramebufferOnly(false);`, add:

   ```rust
   // untagged, P3 screens oversaturate the video (see `window::prepare`)
   super::window::tag_srgb(&layer);
   ```

   If `super::window` does not resolve, find how `src/platform/macos/mod.rs` declares the `window` module and use that path.

## Human check (required)

Build before and after (`git stash` to compare). Play the same SDR video in tbis and in IINA or QuickTime on the built-in display. Look at skin tones and saturated reds.

- Before the change tbis should look more saturated than the reference; after, it should match.
- If the picture after the change is visibly darker or washed out compared to the reference, revert step 2 and write what you saw under Status. Do not guess a different colorspace.

## Done when

- [x] The gate passes
- [ ] A person has compared before and after on a P3 display and recorded the result under Status

**Check:** the gate in `PLAN.md`. Add `runtime unverified` to the Status until the human check is done.
