# 06 — Keyboard shortcuts help

**What to build:** Pressing `?` (and a menu entry) shows an overlay listing every keyboard shortcut, grouped (Playback, Tracks, Navigation), built from the app's actual key bindings so it cannot drift from them. Available in the Player and the main screens.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] `?` opens the overlay; Escape or `?` closes it
- [x] Keys come from the registered key bindings; a new action needs one row in the shortcuts table
- [x] Keys render with gpui-kit's keyboard-key styling
- [x] Works over the Player without pausing playback
