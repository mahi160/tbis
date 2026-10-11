# 04 — One poster click rule

**What to build:** Clicking a poster opens its detail page on every screen (Home, Library, Search). Playback from a poster starts only from the play button that appears on hover. Continue Watching and Next Up cards keep "click the art to play", because resuming is the purpose of those rows. Episode rows keep "the still plays, the row opens". This reverses the Home behaviour chosen in ui-system-cleanup ticket 04 (the Movies and Series row art used to play). The goal is to prevent accidental playback from browsing rows.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [ ] On Home's Movies and Series rows, clicking the poster art opens the detail page
- [ ] Clicking the hover play button on any poster plays it (a Series plays its Next Up Episode, else opens) and doesn't also open the detail page
- [ ] Library and Search posters behave the same as Home posters
- [ ] Continue Watching and Next Up cards still play when the art is clicked
- [ ] On an Episode row, the play button fires once and doesn't also open the Episode
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
