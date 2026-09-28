# 01 — HDR on SDR displays (spike)

**What to build:** Find out how HDR10/HLG files look in tbis today on an SDR display versus `mpv --vo=gpu-next`, and, if they look washed out or wrong, make them tone-map correctly. Record the result (including whether real EDR output is possible with the ADR-0001 render path) in an ADR.

**Blocked by:** None — can start immediately.

**Status:** superseded — investigation completed by `docs/audit-2026-09.html` §4; the ADR and follow-on work continue as `.scratch/audit-2026-09/issues/06-edr-output-adr.md` and `10-edr-output-implementation.md`.

- [ ] Side-by-side comparison against gpu-next on one HDR10 and one HLG file
- [ ] If needed, tone-mapping applied only to HDR content; SDR playback unchanged
- [ ] ADR records the finding and the EDR ceiling (or path)
