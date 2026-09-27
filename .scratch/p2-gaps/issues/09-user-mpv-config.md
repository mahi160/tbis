# 09 — Load the user's own mpv.conf / input.conf

**What to build:** Power users can drop an `mpv.conf` (and optionally scripts/shaders it references) into tbis's support folder and have the in-window Player and PiP apply it. Options tbis needs for embedding (video output, OSC, default bindings — see ADR-0001) are always re-applied after the user config so a user setting can't break the video layer.

**Blocked by:** None — can start immediately.

**Status:** done (in-app runtime unverified)

- [x] `~/Library/Application Support/tbis/mpv/mpv.conf` is read by both the Player and PiP
- [x] Embedding-critical options always win over the user file
- [x] A malformed user config logs a warning and playback still starts (checked with standalone mpv 0.41: bad lines are reported and skipped)
- [x] No config folder present: behaviour identical to today
