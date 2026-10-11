# 08 — Loading and empty states match the layout

**What to build:** While Home loads, it shows skeletons shaped like its rows (a heading bar, a row of wide-card skeletons, then a row of poster skeletons) instead of a poster grid, so the page doesn't jump when content arrives. While a season's Episodes load on Series detail, a few Episode-row skeletons show instead of the text "Loading…". When Home has nothing to show, the message says the server has no movies or series yet.

**Blocked by:** 02 — Series detail leads with playback (changes the Series Episode list)

**Status:** done (visual check pending)

- [ ] Home's loading state is row-shaped and roughly matches the loaded layout
- [x] Library still shows the poster-grid skeleton
- [ ] Switching seasons on Series detail shows Episode-row skeletons until the Episodes arrive
- [x] An empty Home reads "No movies or series on this server yet"
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
