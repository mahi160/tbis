# 08 — Bounded poster memory

**What to build:** The Movies/Series grid keeps every poster it has ever shown in memory (a known limit noted in the code), so memory grows with the size of the Library scrolled. Replace it with a cache that drops the least recently shown posters beyond a fixed budget.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified). Keeps the 200 most recently drawn posters per grid.

- [ ] Scrolling an entire large Library keeps poster memory under a fixed bound
- [ ] Scrolling back to recently seen posters shows them without a visible reload
- [ ] The known-limit note in the code is removed
