# 04 — Media keys and Now Playing

**What to build:** While the Player is open, macOS media controls drive it: keyboard play/pause, AirPods taps, and Control Center / lock-screen buttons play, pause, and skip ±10 s; "next track" plays the next Episode when one exists. Control Center's Now Playing shows the title (Series + Episode label for Episodes), artwork, elapsed time, duration, and playing/paused state, kept current across seeks, pauses, speed changes, and Autoplay. Closing the Player clears Now Playing. PiP keeps responding while it has taken over playback.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] Play/pause/toggle, skip forward/back, and next-track commands work from keyboard media keys and Control Center
- [x] Now Playing shows title, artwork, position, duration, and rate; position stays correct after seek/pause/speed change
- [x] Next-track is disabled when there is no next Episode
- [x] Now Playing is cleared and command handlers removed when the Player closes; while PiP plays, tbis releases Now Playing so the standalone mpv handles media keys itself
- [x] Media keys no longer launch Music.app while tbis is playing
- [x] New macOS-framework dependency added only for macOS targets
