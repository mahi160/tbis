# 07 — One metadata layout on detail headers

**What to build:** Detail headers show metadata in a single, predictable way. One muted text line gives year, runtime or season count, star score and Critics %. One pill row follows, with the age rating first and then format tags (resolution, HDR, audio). Genres stay a caption line. Header order: title, meta line, genres, pill row, overview, actions.

**Blocked by:** 03 — Detail pages: back row and one left edge; 06 — Visual hierarchy (same header, final type sizes)

**Status:** done (visual check pending)

- [x] The age rating no longer appears as a pill inside the meta line
- [x] The age rating and format tags share one pill row, age rating first
- [x] Missing parts are skipped without leaving gaps or empty rows
- [x] Movie, Episode and Series detail all follow the same order
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
