# 10 — Implement real EDR/HDR output on CAMetalLayer

**What to build:** Following the design in ticket 06's ADR, switch the video `CAMetalLayer` to the chosen extended-range pixel format, set `wantsExtendedDynamicRangeContent`, feed it the display's actual EDR headroom, and point mpv at that target via `target-trc`/`target-peak`. HDR10/HLG content reaches the screen in extended range on capable displays instead of being clipped to 8-bit SDR — the one substantial engineering bet in `docs/audit-2026-09.html`, ahead of IINA's still-open equivalent request and of Moonfin's non-mpv macOS path (§4.4, §8).

**Blocked by:** 06 — ADR: real EDR/HDR output on CAMetalLayer.

**Status:** ready-for-agent

- [ ] HDR10 (PQ) and HLG test files show extended-range output on an EDR-capable display, matching the ADR's design
- [ ] SDR playback is pixel-identical to before this change
- [ ] Display headroom updates correctly when the window moves to a different display
- [ ] Falls back to today's SDR clipping behaviour on a non-EDR display without artifacts or crashes
- [ ] Playback-info overlay's HDR badge (ticket 02) reflects real output instead of "tone-mapped to SDR" once this lands
