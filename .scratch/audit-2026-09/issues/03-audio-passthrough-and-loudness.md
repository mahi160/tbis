# 03 — Audio passthrough + loudness normalization settings

**What to build:** Settings > Playback > Audio gains two independent toggles: bitstream passthrough (mpv's `--audio-spdif=ac3,dts,eac3` plus `--audio-exclusive=yes` for CoreAudio exclusive mode on macOS) for AV receivers, and loudness normalization / "night mode" (an `af` filter chain such as `dynaudnorm`/`loudnorm`). Both are plain mpv options/filters behind a settings switch, no new architecture (`docs/audit-2026-09.html` §5, §8).

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] Passthrough toggle in Settings > Playback > Audio, saved to config, applied at the next playback start (Player and PiP)
- [x] Loudness normalization toggle in the same section, same application path
- [x] `audio-spdif`/`audio-exclusive` are mpv's own passthrough options; mpv already falls back to normal decode itself when the receiver/format doesn't support the bitstream
- [x] Independent boolean fields; either can change without touching the other, and both are plain config + mpv options like the HDR settings
