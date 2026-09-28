# 01 — HDR-source badge in playback info overlay

**What to build:** The existing playback-info overlay already reads `video-params/primaries` and `video-params/gamma` and shows them as a raw string. Replace that with a readable badge — e.g. "HDR (PQ) → tone-mapped to SDR" or "HDR (HLG) → tone-mapped to SDR" — so the user knows their HDR source isn't being wasted, ahead of real EDR output existing (`docs/audit-2026-09.html` §4.4, §8).

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] SDR sources show no HDR badge, unchanged raw `primaries / gamma` string as before
- [x] PQ (`video-params/gamma == pq`) sources show "HDR (PQ) → tone-mapped to SDR"
- [x] HLG (`video-params/gamma == hlg`) sources show "HDR (HLG) → tone-mapped to SDR"
- [x] Recomputed every poll tick like the rest of the overlay, so it updates live on track switch or new file
