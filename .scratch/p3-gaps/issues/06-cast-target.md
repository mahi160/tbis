# 06 — Cast target ("Play on tbis")

**What to build:** tbis registers as a remote-controllable Jellyfin session, so from Jellyfin web or the mobile app the user can pick "tbis" and start, pause, seek, stop, and change tracks/volume on it. Incoming play commands open the Player on the requested item and position.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] tbis appears as a cast target in Jellyfin web/mobile while running and signed in
- [ ] Remote play, pause, seek, stop, next/previous, volume, and track changes act on the Player
- [ ] Playback state reported back so the remote shows correct position
- [ ] Connection drops reconnect automatically without user action
