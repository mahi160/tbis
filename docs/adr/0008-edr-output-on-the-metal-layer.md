---
status: accepted
---

# Real EDR output: an RGBA16F target, extended-linear Display P3, switched on only for HDR files

This builds on ADR-0001. Today the video `CAMetalLayer` is `BGRA8Unorm`, the IOSurface behind mpv's GL framebuffer is `'BGRA'`, and mpv tone-maps every HDR10/HLG file down to SDR. When the file is HDR and the window's display has extended dynamic range (EDR) headroom, the surface switches to a half-float target that carries values above 1.0 to the screen. mpv is told the display's real peak instead of an SDR one. Otherwise nothing changes.

The platform investigation (why `--target-colorspace-hint` does not reach macOS, why Moonfin bypasses mpv there, IINA's open request) is in `docs/audit-2026-09.html` §4 and §8. This record is only the decision.

**Render target.** Both ends use half-float RGBA: the IOSurface is created with pixel format `'RGhA'` (`kCVPixelFormatType_64RGBAHalf`, 8 bytes per element), GL attaches it with `CGLTexImageIOSurface2D(GL_TEXTURE_RECTANGLE, GL_RGBA16F, …, GL_RGBA, GL_HALF_FLOAT, …)`, and Metal wraps the same surface as `MTLPixelFormat::RGBA16Float`. The layer is `RGBA16Float` too, so the existing `copyFromTexture_toTexture` blit stays a same-format copy (a blit cannot convert formats). `make_target` in `src/platform/macos/video.rs` is the one place that picks the format, next to the existing `layer.setPixelFormat`.

**Layer colorspace and metadata.** `wantsExtendedDynamicRangeContent = true` and `colorspace = kCGColorSpaceExtendedLinearDisplayP3`. Pixel values are linear light in Display P3 primaries, where 1.0 is SDR reference white and anything above is headroom. macOS tone-maps nothing and does no PQ/HLG decoding, because mpv already produced display-referred light. No `CAEDRMetadata` is set. The alternative, `kCGColorSpaceITUR_2100_PQ`/HLG plus `EDRMetadata`, makes mpv emit the source signal and hands the display mapping to the OS. That needs a 10-bit-or-wider PQ/HLG surface, and macOS then applies its own opaque tone mapping with no mpv-side control, so a user's `tone-mapping` option would stop applying. Every symbol above is in the objc2 bindings already in the lockfile (`CAMetalLayer::setWantsExtendedDynamicRangeContent`, `setColorspace`, `kCGColorSpaceExtendedLinearDisplayP3`).

**mpv output.** For HDR files on an EDR display: `target-trc=linear`, `target-prim=display-p3`, and `target-peak` set to the current headroom times 203 (nits; BT.2408 reference white), so a headroom of 4.0 means `target-peak=812`. mpv's tone-mapping then compresses only what exceeds the display's real peak, instead of the SDR ceiling it assumes now. mpv does not pick any of this up from the layer on macOS, so tbis sets it explicitly.

**Headroom.** `NSScreen.maximumExtendedDynamicRangeColorComponentValue` is the current headroom. macOS lowers it with brightness and ambient light and reports 1.0 when no EDR content is on screen. `maximumPotentialExtendedDynamicRangeColorComponentValue` is the ceiling the display can reach. The two are used differently:

- Eligibility uses the potential value of the window's screen: the HDR path is available only when it is above 1.0.
- `target-peak` uses the current value, floored at 1.0, read when the window's screen changes (`NSWindowDidChangeScreenNotification`, plus `NSApplicationDidChangeScreenParametersNotification` for hot-plug and mode changes) and then re-read on each presented frame. The main thread owns the `NSScreen` lookup and publishes the value; the render thread reads it. `target-peak` is re-sent only when the value moves by more than about 5%, so a brightness slider does not re-run mpv's tone-mapping every frame.

**Switching.** The format change is made only when the file's `video-params/gamma` is `pq` or `hlg` (the same test the playback-info overlay already uses) and eligibility holds. The target is rebuilt in the new format when such a file loads and rebuilt in `BGRA8Unorm` when the next file is SDR, the same rebuild path a size change already takes. SDR files never touch the float path, so SDR output stays pixel-identical to today. Leaving the float format on for SDR was rejected: 1.0 in extended-linear is not guaranteed to land on the same 8-bit values after the display pipeline, and SDR playback would cost twice the bandwidth for no gain.

**Interplay with tone-mapping options.** Ticket 02's curve and deband presets have since been replaced by the user's own mpv options in Settings (`tone-mapping=`, `deband=`). They stay in force: on an EDR display the curve now shapes only the part above the display's real peak, and on a non-EDR display it behaves as before. They are not forked per path. The playback-info overlay's `HDR (…) → tone-mapped to SDR` line must describe the real output once the HDR path is active; ticket 10 covers it.

**Scope.** HDR10 (PQ) and HLG only. HDR10+ dynamic metadata is whatever mpv does with it; nothing here reads it. Dolby Vision is out of scope (audit §8): profile 5 and similar are left to mpv's existing handling, and DV files with an HDR10 base layer play as HDR10. Linux and Windows are unchanged. ADR-0007 is a separate path.

**Fallback.** On a display with no EDR potential (potential value 1.0), or when the float IOSurface/FBO cannot be created, tbis keeps today's `BGRA8Unorm` path and mpv's SDR tone-mapping. Failing to create the float target is not an error: it logs once and takes the SDR path.

## Considered Options

- **PQ/HLG layer colorspace with `CAEDRMetadata`**: rejected, see above. It removes mpv-side tone-mapping control and relies on macOS's mapping.
- **`RGB10A2Unorm` / `BGR10_XR`**: rejected. `BGR10_XR` has no OpenGL internal format (Metal-only), so mpv's GL framebuffer could not render into the shared IOSurface. `RGB10A2` is not extended-range, so it cannot exceed 1.0.
- **Always-float target**: rejected, see Switching.
- **Bypassing mpv's render API with AVSampleBufferDisplayLayer (Moonfin's approach)**: rejected because we lose subtitles, shaders and our own controls over mpv (ADR-0001).

## Consequences

- The IOSurface roughly doubles in size for HDR files (8 bytes per pixel against 4). At 3420×2148 that is about 59 MB per target, with the target count unchanged. The 1.2–1.8 ms drawable copy in ADR-0001 was measured at 4 bytes per pixel and has to be re-measured.
- A new objc2 feature set is needed: `objc2-quartz-core` for the layer calls, and `objc2-core-graphics` (already a dependency) for the colorspace. No new crate.
- Unverified until ticket 10 runs on hardware: that `CGLTexImageIOSurface2D` accepts `GL_RGBA16F` against an `'RGhA'` surface, that mpv's `target-trc=linear` is normalised to 203 nits (the `203` scale in `target-peak` depends on it), and that the extended-linear colorspace shows no banding or clipping on the target display.
- Open question: if the GL/IOSurface combination fails on a real machine, the fallback is a Metal-only blit through a GL `RGBA16F` renderbuffer copy, which costs one more copy per frame. That would amend this record.
