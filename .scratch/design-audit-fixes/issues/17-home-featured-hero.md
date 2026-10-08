# 17 — Home featured item

**What to build:** Home opens with a featured item above the rows: the most recent Continue Watching item, else the first Next Up Episode, else the newest unstarted Movie. It shows the item's backdrop (the Series backdrop for an Episode) with a readable scrim, the title (Series name plus Episode code and name for an Episode), a short meta line, a progress bar when it's been started, and two buttons: Play or Resume, and Details. Play/Resume starts the Player, and Details opens the detail page. The item also stays in its row. Home's loading skeleton includes a hero-shaped block. With nothing to feature, Home has no hero.

**Blocked by:** 13 — Format the branch

**Status:** done (visual check pending)

- [x] The hero picks Continue Watching first, then Next Up, then the newest Movie
- [x] An Episode shows its Series backdrop, Series name and Episode code; it never shows an unwatched Episode's still when spoilers are hidden
- [x] Resume shows when the item has a saved position, otherwise Play; Details opens the detail page
- [ ] Title, meta and buttons stay readable on bright and dark backdrops in every theme
- [ ] The hero fits the window's minimum size without pushing every row off-screen
- [x] Home's loading skeleton has a hero-shaped block; with no featured item, no hero renders
- [ ] Screenshots of Home with each hero source
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
