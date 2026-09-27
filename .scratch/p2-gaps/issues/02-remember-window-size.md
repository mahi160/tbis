# 02 — Remember window size and position

**What to build:** tbis reopens at the size, position, and maximized state it had when last closed, instead of always 1200×800 centered. If the saved position is off every current display (monitor unplugged), it falls back to centered.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Size, position, and maximized state saved on quit/close and restored on launch
- [ ] Off-screen saved bounds fall back to centered default size
- [ ] Fullscreen at quit restores as the pre-fullscreen window, not fullscreen
- [ ] Minimum window size still enforced
