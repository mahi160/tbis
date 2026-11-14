# 14 — Thumbnail preview while hovering the timeline

**What to build:** The trickplay thumbnail appears only while dragging the seek slider. Show it, with the time under the pointer, while the mouse hovers the timeline too.

**Model:** sonnet

**Blocked by:** 13 (both edit `timeline`; do them in order to avoid conflicts).

**Status:** blocked

**Read first:** `src/player/controls/chrome.rs` (`fn bottom_chrome`, `fn trickplay_preview`), `src/components/video.rs` (`timeline`), `src/player/playback.rs` (`scrubbing`), gpui's `on_mouse_move` and `MouseMoveEvent` in `~/.cargo/registry/src/*/gpui-pre-0.3.9/src/`.

## Steps

1. Add `hover_time: Option<f64>` to the Player's playback state.
2. The preview needs the pointer's fraction along the slider. In `timeline`, wrap the slider container in an element that reports its bounds; gpui's `canvas` element or `on_mouse_move` with the element's bounds both work. Read how `Slider` in gpui-kit computes its own fraction and reuse that approach. Add two callbacks to `timeline`: `on_hover(fraction: f32)` and `on_leave()`.
3. In `fn bottom_chrome`, the callbacks set `hover_time = Some(fraction * duration)` and `None`, each followed by `cx.notify()`.
4. Compute the preview from `self.playback.scrubbing.or(self.playback.hover_time)` instead of `scrubbing` alone, and position it at that time's fraction (today it is positioned at the playhead's).
5. Under the thumbnail show the time with `format_time`. When the item has no trickplay data, show the time alone.

## Done when

- [ ] Hovering the timeline shows the thumbnail and time for the hovered position
- [ ] Leaving the timeline hides it
- [ ] Dragging still shows the preview as before
- [ ] Items without trickplay show the time only

**Check:** the gate in `PLAN.md`, then hover and drag on an item with trickplay and one without.
