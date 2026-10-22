# 03 — Resize the video only when the window size or scale changes

**What to build:** Every Player (and PiP) render hands the window size to mpv and wakes its render thread, even when nothing changed. Only pass a new size when the viewport size or scale factor actually differs from the last one.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Re-rendering at an unchanged size does not wake mpv's render thread
- [ ] Resizing the window, entering/leaving fullscreen, moving between displays with different scale, and entering/leaving PiP still resize the video correctly
