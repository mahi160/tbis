# 02 — Frosted, immersive Series detail

**What to build:** Series detail gets the same immersive frosted treatment as Movie detail: a fixed, blurred, theme-tinted Series backdrop behind the page, a faint sharp top, and frosted cards. The header (with the Play/Resume button), the season tabs together with the Episode list, and the cast each sit in their own card. Scroll position is still kept when coming back from an Episode, and Episode rows keep their hover fill, which must stay visible on the frosted card.

**Blocked by:** 01 — Frosted, immersive Movie and Episode detail

**Status:** done (visual check pending)

- [x] Series detail uses the same backdrop layers and frosted cards as Movie detail, with no duplicated code
- [x] Season tabs and Episodes share one card; header and cast have their own
- [ ] Episode row hover, the play scrim and the watched check still read clearly on the frosted card
- [ ] Back from an Episode returns to the same scroll position
- [ ] Screenshots of Series detail in Raptor Night and Fossil Slate
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
