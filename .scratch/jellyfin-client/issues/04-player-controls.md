# 04 — Player controls

**What to build:** The Player gains an audio track picker and a subtitle track picker (including "off"), built from mpv's own track list with no server transcode. It also gets a native macOS fullscreen toggle, a volume slider, and mute. Keyboard shortcuts: Space toggles play/pause, ←/→ seek ±10s, F toggles fullscreen, M toggles mute, and Esc leaves fullscreen.

**Blocked by:** 03 — Player tracer bullet

**Status:** done

- [ ] Switching the audio track changes the audio without restarting playback
- [ ] Switching subtitles, including image-based subtitles, works, and "off" hides them
- [ ] Fullscreen enters and exits through the button, F, and Esc
- [ ] Volume and mute work from the controls and from M
- [ ] Space and ←/→ behave as described
