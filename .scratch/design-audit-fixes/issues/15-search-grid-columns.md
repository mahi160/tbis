# 15 — Search posters use Library's column layout

**What to build:** Search's Movies and Series results lay out in the same columns as the Library grid: at a given window width, a Search poster is the same width as a Library poster and the row fills the width edge to edge. The column math lives in one shared place used by both screens.

**Blocked by:** 13 — Format the branch

**Status:** done (visual check pending)

- [ ] At the same window width, Search and Library posters are the same width
- [ ] Search result rows fill the content width with no ragged right gap
- [x] The column calculation exists once and both screens use it
- [ ] Resizing the window reflows Search results like Library
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
