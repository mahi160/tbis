# 02 — Series detail leads with playback

**What to build:** Series detail has a primary Play or Resume button for the Series' Next Up Episode, labelled with its code (for example `Resume S1E3`). It plays that Episode in the Player and stays correct after the user comes back from the Player. When the Series has no Next Up Episode, the button is hidden. Episodes come right after the season tabs, and cast moves to the bottom of the page, the same as Movie detail. The overview gets the same line-length cap as on Movie detail.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [ ] The button reads Resume when the Next Up Episode has a saved position, otherwise Play, with the Episode code
- [ ] After watching an Episode and returning, the button points at the new Next Up Episode
- [ ] A fully played Series shows no Play button
- [ ] Page order: header, season tabs, status, Episodes, cast
- [ ] On a wide window, the overview wraps at the same width as on Movie detail
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
