# 05 — Measure, then remove the extra GPU copy on macOS

**What to build:** On macOS, mpv renders into an IOSurface-backed texture that is then copied into a Metal drawable every frame (about 33 MB per frame at 4K), and getting the drawable can block the render thread (see ADR-0001). First measure GPU time and frame pacing for 1080p and 4K playback in Instruments. If the copy is a real cost, show mpv's frame directly (e.g. the IOSurface as layer contents, with a small surface pool) so no copy is made. If it is not, record the measurement and the decision to keep the copy in an ADR note.

**Blocked by:** None — can start immediately.

**Status:** needs-human: the first step is an Instruments measurement during real 1080p/4K/HDR playback, which an agent can't run.

- [ ] Before measurements (GPU time per frame, dropped frames) recorded for 1080p and 4K, including HDR
- [ ] Either: the copy is gone and after measurements show the improvement with no tearing, flicker or dropped frames, in the Player, fullscreen and PiP
- [ ] Or: an ADR note records the measurements and why the copy stays
