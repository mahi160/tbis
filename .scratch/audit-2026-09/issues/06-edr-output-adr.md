# 06 — ADR: real EDR/HDR output on CAMetalLayer

**What to build:** A decision record, following the ADR-0001..0004 convention, that settles the design for getting real extended-range pixels to the screen instead of today's 8-bit SDR `BGRA8Unorm` `CAMetalLayer`. Covers: the render-target format (a float/XR pixel format such as `rgba16Float`), `wantsExtendedDynamicRangeContent`, reading the display's actual headroom (`NSScreen.maximumExtendedDynamicRangeColorComponentValue`) per frame, and pointing mpv at that ceiling via `target-trc`/`target-peak` instead of clipping to SDR. Builds directly on `docs/audit-2026-09.html` §4, which already did the platform-compatibility investigation (why `--target-colorspace-hint` doesn't reach macOS, why Moonfin bypasses mpv there, IINA's own unresolved gap) — this ticket is the decision, not a repeat of that research.

Supersedes `.scratch/p3-gaps/issues/01-hdr-tonemap-spike.md`, whose investigation the audit's §4 deep dive already completed; that ticket's status should be marked superseded by this one.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] ADR records the chosen render-target format and colorspace/metadata handling
- [ ] ADR records how per-display EDR headroom is read and kept current across display changes
- [ ] ADR records the mpv-side options needed (`target-trc`/`target-peak` or equivalent) and how they interact with tone-mapping settings from ticket 01
- [ ] ADR states the ceiling/scope explicitly (e.g. HDR10/HLG only, Dolby Vision profile handling out of scope per audit §8)
