# 04 — Playback Info overlay

**What to build:** A toggleable overlay in the Player (shortcut plus an entry in the control bar menu) showing live playback stats read from mpv: container, video codec/resolution/fps, HDR/colour info, audio codec/channels, current bitrate, hardware decoding in use, dropped/delayed frames, cache/buffer ahead. Updates about once a second while open.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] Shortcut (`i`) and a control-bar button toggle the overlay; it survives controls auto-hiding
- [x] Fields update live and show "—" when mpv doesn't report a value
- [x] Overlay stays legible over any video frame and doesn't block controls
- [x] Polling stops when the overlay closes or the Player closes
