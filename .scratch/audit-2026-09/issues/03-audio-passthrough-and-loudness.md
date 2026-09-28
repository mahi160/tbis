# 03 — Audio passthrough + loudness normalization settings

**What to build:** Settings > Playback > Audio gains two independent toggles: bitstream passthrough (mpv's `--audio-spdif=ac3,dts,eac3` plus `--audio-exclusive=yes` for CoreAudio exclusive mode on macOS) for AV receivers, and loudness normalization / "night mode" (an `af` filter chain such as `dynaudnorm`/`loudnorm`). Both are plain mpv options/filters behind a settings switch, no new architecture (`docs/audit-2026-09.html` §5, §8).

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Passthrough toggle in Settings > Playback > Audio, saved to config, applied on playback start
- [ ] Loudness normalization toggle in the same section, saved to config, applied on playback start
- [ ] Passthrough on a receiver that doesn't support the bitstream falls back to normal decode without crashing playback
- [ ] Both toggles can be changed independently and take effect on the next played item
