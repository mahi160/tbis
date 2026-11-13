# 09 — AirPlay-out casting

**What to build:** Playback can be routed to an AirPlay receiver instead of the local window, as a lighter alternative to full casting/DLNA support, which stays out of scope (`docs/audit-2026-09.html` §3, §8).

**Blocked by:** None — can start immediately.

**Status:** wontfix — mpv draws its own video, so macOS could only route audio to AirPlay; video AirPlay would mean replacing mpv with AVPlayer.

- [ ] An AirPlay output picker is reachable from the Player
- [ ] Selecting a receiver routes audio/video to it and playback controls keep working
- [ ] No AirPlay receivers on the network: the option is absent or clearly disabled
- [ ] Returning to local playback works from the same picker
