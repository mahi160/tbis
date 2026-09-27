# 08 — Screenshot key

**What to build:** A Player shortcut saves the current video frame (without controls or subtitles overlays drawn by tbis) as an image to the user's Pictures (or Desktop) folder, named after the item and timestamp, and confirms with a toast.

**Blocked by:** 01 — Toasts for transient messages.

**Status:** done (runtime unverified)

- [x] Shortcut saves a full-resolution frame via mpv
- [x] File name includes item title and playback position
- [x] Toast confirms success or shows the failure reason
- [x] Works in fullscreen and while paused
