# 02 — Tone-mapping curve + debanding settings

**What to build:** Settings > Playback > Video gains a tone-mapping curve picker (bt.2390 / hable / spline / mobius / reinhard, plus mpv's default) and a debanding preset (off / weak / medium / strong). Both apply live on the running mpv `Handle` via `set_property`, the same pattern the existing shader-profile toggle already uses. mpv already tone-maps HDR to SDR by default today — this exposes the curve choice and adds debanding for the banding that shows up in tone-mapped gradients, per `docs/audit-2026-09.html` §4.4 and §8.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Tone-map curve and deband preset pickers in Settings > Playback > Video, saved to config
- [ ] Selection applies on playback start and takes effect immediately if changed mid-playback
- [ ] Only affects HDR-tone-mapped content; SDR playback is visually unchanged
- [ ] An unsupported/rejected value falls back to mpv's default curve without breaking playback
