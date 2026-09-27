# 12 — Remote bitrate cap / transcode fallback

**What to build:** A Playback section in Settings sets a maximum streaming bitrate (with "Unlimited" as default for the local network). When the item's bitrate exceeds the cap, tbis asks the server for a transcoded stream at that bitrate instead of direct play. The Player shows (e.g. in Playback Info) whether it is direct playing or transcoding.

**Blocked by:** p1-gaps 06 — Settings screen with default language.

**Status:** ready-for-agent

- [ ] Bitrate cap setting saved to config; default keeps today's always-direct-play behaviour
- [ ] Over-cap items play via a server transcode at or under the cap
- [ ] Resume position, track picks, and progress reporting still work on transcoded streams
- [ ] Transcode session is stopped on the server when playback ends
