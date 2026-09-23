# 03 — Player tracer bullet

**What to build:** Clicking a Movie opens the Player in the same window, and mpv video is composited as a native layer under a transparent gpui window (ADR-0001). Playback is always direct: the original file is streamed with no transcoding. If the Movie has a saved position, playback silently starts there. The gpui controls on top are play/pause, a seek bar with current and total time, and Back. Playback start, periodic progress, pause, and stop are reported to Jellyfin, so the position and played state sync. When the Movie reaches the end, it is marked played and the app returns to the previous screen.

This is the risk ticket. If the mpv layer cannot be composited under gpui's view, stop and report back before trying any alternative.

**Blocked by:** 02 — Movies page

**Status:** ready-for-agent

- [ ] Video renders inside the main window with the gpui controls visible on top
- [ ] No second window or spawned process is used
- [ ] Other screens keep an opaque window, and only the Player is transparent
- [ ] A partly watched Movie resumes at its saved position
- [ ] Seeking, pausing, and Back work
- [ ] The Jellyfin web UI shows the updated position after stopping midway
- [ ] Watching to the end marks the Movie played and returns
