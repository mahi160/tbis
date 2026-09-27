# 12 — Remote bitrate cap / transcode fallback

**What to build:** A Playback section in Settings sets a maximum streaming bitrate (with "Unlimited" as default for the local network). When the item's bitrate exceeds the cap, tbis asks the server for a transcoded stream at that bitrate instead of direct play. The Player shows (e.g. in Playback Info) whether it is direct playing or transcoding.

**Blocked by:** p1-gaps 06 — Settings screen with default language.

**Status:** done (runtime unverified against a real transcode)

- [x] Bitrate cap setting saved to config; default keeps today's always-direct-play behaviour
- [x] Over-cap items play via a server transcode at or under the cap
- [x] Resume position and progress reporting work on transcoded streams (transcode requested from 0 so mpv's resume seek and reports stay on the file's timeline; reports carry the server's PlaySessionId and PlayMethod Transcode)
- [ ] Track picks on transcoded streams: the server's HLS output carries only the audio track it negotiated, so switching audio mid-transcode needs a fresh PlaybackInfo negotiation. Not built yet.
- [x] Transcode session is stopped on the server when playback ends (DELETE /Videos/ActiveEncodings after Stopped)
