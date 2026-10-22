---
status: accepted
---

# Embed mpv as a native layer under a transparent gpui window

Playback runs libmpv in-process. There is no spawned mpv process and no second window. mpv renders through its OpenGL render API into an IOSurface-backed framebuffer, which is presented by a `CAMetalLayer` in an `NSView`. That view is added to the window's `contentView` below gpui's own view. When the Player is shown, the gpui window switches to a transparent background, and gpui draws the playback controls on top of the video.

## Considered Options

- **Spawned mpv window**: rejected because we want one window and our own controls.
- **gpui `surface()` element**: rejected. On macOS it only accepts 8-bit NV12 `CVPixelBuffer`s, and mpv outputs RGB. That would add an RGB→NV12→RGB round trip with 4:2:0 chroma loss and no 10-bit/HDR, and gpui would redraw the whole scene on every video frame. Revisit this only if video has to live inside normal layouts (for example, hover previews).
- **libmpv software render into gpui images**: rejected because it is too slow for 4K/HDR.

## Consequences

Each frame is copied from mpv's IOSurface into the layer's drawable. Measured with `TBIS_FRAME_STATS=1` on a 4K Dolby Vision HEVC file at 24 fps, fullscreen at 3420×2148 with VideoToolbox decoding, the copy takes 1.2–1.8 ms of GPU time per frame (max about 9 ms), and waiting for a drawable takes under 0.1 ms. No frames dropped in steady playback. Presenting the IOSurface directly would save only that copy, so we keep the copy. The ~41 ms "mpv GL render" figure in the same log is mostly not render cost: `mpv_render_context_render` blocks until each frame's display time.

This decision covers macOS only. Linux and Windows each need their own surface later. On Linux, gpui paints the whole X11/Wayland window through wgpu. An X11 child window would sit above that paint, and a Wayland subsurface would need gpui's own `wl_display`. This is the same failure class as Photon's ADR-0010. On Windows, gpui renders through DirectComposition, so an mpv visual beneath gpui's visual looks plausible but is unverified.
