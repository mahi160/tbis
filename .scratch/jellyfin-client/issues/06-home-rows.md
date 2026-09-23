# 06 — Home rows

**What to build:** Home shows four horizontal rows, with every row covering all Libraries:
- **Continue Watching**: Movies and Episodes the user has started but not finished.
- **Next Up**: the next unwatched Episode of each Series the user is following, minus anything already in Continue Watching.
- **Movies row**: unstarted Movies, with the most recently added first.
- **Series row**: Series that are not fully played, ordered by when their newest Episode was added.

Continue Watching and Next Up cards use a 16:9 thumbnail, the title, `S2E3 · Episode name` for Episodes, and a progress bar. Movies-row and Series-row cards use the 2:3 poster style. Clicking a Movie or Episode card plays it, and clicking a Series card opens Series detail. Home refreshes after returning from the Player.

**Blocked by:** 03 — Player tracer bullet, and 05 — Series page and Series detail

**Status:** ready-for-agent

- [ ] Each row shows the right items in the right order
- [ ] A partly watched Episode appears in Continue Watching but not in Next Up
- [ ] A partly watched Movie appears in Continue Watching but not in the Movies row
- [ ] Clicks play the item or open Series detail as described
- [ ] Stopping playback midway and returning updates Continue Watching
