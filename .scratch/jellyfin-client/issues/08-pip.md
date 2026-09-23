# 08 — PiP

**What to build:** Pressing P, or clicking the PiP button in the Player, pauses the Player and hands playback to a standalone mpv window (ADR-0003). The window is small, borderless, always on top, visible on all Spaces, and starts at the current position. When the PiP window closes, or P is pressed again, the Player resumes at the position PiP reached.

**Blocked by:** 03 — Player tracer bullet

**Status:** ready-for-agent

- [ ] P opens PiP at the current position, and the Player pauses
- [ ] The PiP window stays on top across Spaces
- [ ] Closing PiP resumes the Player at PiP's position
- [ ] Pressing P again closes PiP and resumes the Player
- [ ] Only one of the two plays audio at any moment
