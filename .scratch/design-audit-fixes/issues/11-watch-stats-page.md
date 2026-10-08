# 11 — Watch stats as its own page

**What to build:** Watch stats are not a setting. They move out of Settings into a "Watch stats" item in the title-bar user menu. It opens as its own page with a Back button, like Settings. The stats content (totals, the weekly chart, most-watched Series) is unchanged.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [x] Settings no longer has a Stats page
- [ ] The user menu has Watch stats, and it opens a page with Back and the same stats content
- [x] Back returns to wherever the user was before
- [ ] Loading, empty and error states of the stats still show
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
