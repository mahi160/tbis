# 03 — Detail pages: back row and one left edge

**What to build:** On Movie, Episode and Series detail, the Back button sits in its own row above the header, as it already does on Settings, instead of inline before the poster. The poster, cast, season tabs and Episodes then all start at the same page gutter.

**Blocked by:** 02 — Series detail leads with playback (both change the Series header)

**Status:** done (visual check pending)

- [ ] Back appears above the header on Movie, Episode and Series detail
- [ ] The poster's left edge lines up with the cast grid and the Episode list
- [x] Back still pops one detail page (Episode back to its Series)
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
