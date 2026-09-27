# 04 — Watch stats

**What to build:** A Stats section in Settings shows a read-only summary of the user's viewing from the server: time watched this week/month/all time, Movies and Episodes finished, most-watched Series, and a simple activity chart.

**Blocked by:** p1-gaps 06 — Settings screen with default language.

**Status:** done (runtime unverified)

- [x] Stats computed from server play data, not only local sessions (the server keeps only last-play date and play count per item, so 7/30-day totals and the weekly chart count each item once, at its last play)
- [x] Loading/empty/error states use the shared status component
- [x] Chart uses gpui-kit's chart component and theme colours
