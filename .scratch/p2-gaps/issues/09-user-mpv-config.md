# 09 — Load the user's own mpv.conf / input.conf

**What to build:** Power users can drop an `mpv.conf` (and optionally scripts/shaders it references) into tbis's support folder and have the in-window Player and PiP apply it. Options tbis needs for embedding (video output, OSC, default bindings — see ADR-0001) are always re-applied after the user config so a user setting can't break the video layer.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] A documented config folder under tbis's support directory is read by both the Player and PiP
- [ ] Embedding-critical options always win over the user file
- [ ] A malformed user config logs a warning and playback still starts
- [ ] No config folder present: behaviour identical to today
