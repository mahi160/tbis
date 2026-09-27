# 07 — mpv network cache tuning

**What to build:** Streaming from the server gets a real read-ahead buffer so brief network hiccups don't stall playback and seeks within the buffered range are instant. Buffer sizes are chosen to stay within reasonable memory for 4K remuxes.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] mpv cache enabled with explicit forward/back buffer limits and read-ahead duration
- [ ] Playback of a large 4K remux over the LAN starts no slower than today
- [ ] Seeking back a few seconds does not re-request from the server
- [ ] Memory use of a long 4K session stays bounded (checked in Activity Monitor)
