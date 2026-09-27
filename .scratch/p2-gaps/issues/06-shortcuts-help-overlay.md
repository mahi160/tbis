# 06 — Keyboard shortcuts help

**What to build:** Pressing `?` (and a menu entry) shows an overlay listing every keyboard shortcut, grouped (Playback, Tracks, Navigation), built from the app's actual key bindings so it cannot drift from them. Available in the Player and the main screens.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] `?` opens the overlay; Escape or `?` closes it
- [ ] List is generated from the registered key bindings, including any added later
- [ ] Keys render with gpui-kit's keyboard-key styling
- [ ] Works over the Player without pausing playback
