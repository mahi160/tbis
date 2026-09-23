# 05 — Series page and Series detail

**What to build:** The Series tab shows a poster grid of every Series from all Libraries. It uses the same card style, sort dropdown, and watched badge as the Movies page, where a Series counts as watched when it is fully played. Clicking a Series opens Series detail, which has a season picker and an Episode list. The picker opens on the season that holds the Series' Next Up Episode, falling back to the first season. Each Episode row shows a thumbnail, number, name, runtime, a played check, and a progress bar. Clicking an Episode plays it in the Player, resuming silently when it has a saved position.

**Blocked by:** 02 — Movies page, and 03 — Player tracer bullet

**Status:** ready-for-agent

- [ ] Series from every Library appear, and every sort option works
- [ ] Fully played Series show the check badge
- [ ] Series detail opens on the Next Up season, or the first season when there is none
- [ ] Episode rows show their played and progress state
- [ ] Clicking an Episode plays it, and Back returns to Series detail
