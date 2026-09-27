# 05 — Richer detail pages

**What to build:** Movie detail and Series detail show more of the server's metadata in the meta line and below the overview: community rating (star) and critics score (%), official age rating, runtime (Movie) or season/Episode count (Series), genres, and the main cast (first several people with role, portrait when available). Missing fields are simply omitted.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] The detail request asks the server for the extra fields; list pages keep their lighter requests
- [x] Ratings, age rating, runtime/counts, and genres appear in the meta area when present
- [x] A cast row shows the top billed people with name, role, and portrait (placeholder initials when no image)
- [x] Items with none of these fields look the same as today (no empty sections)
- [x] Colors and type come from the theme; reuses existing card/row components where they fit
