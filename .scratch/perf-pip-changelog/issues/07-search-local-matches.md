# 07 — Search typing stays instant on large libraries

**What to build:** On every keystroke, before the 250 ms debounce, Search scans every loaded item and lowercases its name to find local matches. Lowercase names once when the Library loads, or move the local scan behind the debounce, so typing stays instant on large libraries.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] A keystroke no longer lowercases every loaded item name
- [ ] Search results (local and server-side) are the same as before for the same query
- [ ] Clearing the query still returns to the previous screen as before
