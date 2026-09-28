# 02 — Tone-mapping curve + debanding settings

**What to build:** Settings > Playback > Video gains a tone-mapping curve picker (bt.2390 / hable / spline / mobius / reinhard, plus mpv's default) and a debanding preset (off / weak / medium / strong). Both apply live on the running mpv `Handle` via `set_property`, the same pattern the existing shader-profile toggle already uses. mpv already tone-maps HDR to SDR by default today — this exposes the curve choice and adds debanding for the banding that shows up in tone-mapped gradients, per `docs/audit-2026-09.html` §4.4 and §8.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] Tone-map curve and deband preset pickers in Settings > Playback > HDR, saved to config
- [x] Selection applies at the next playback start (Player and PiP); Settings, like other playback options, isn't reachable mid-playback
- [x] `tone-mapping`/`deband*` are plain mpv options; SDR playback is unaffected by the curve choice, only by debanding if enabled
- [x] Unknown/corrupted config values fall back to their `#[default]` variant (`Auto` / `Off`) via serde
