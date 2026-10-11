# 05 — Back and tab shortcuts

**What to build:** Outside the Player, `⌘[` and the mouse back button go back. That closes the current detail page or Settings, or clears an active Search when no detail page is open. `⌘1`, `⌘2` and `⌘3` switch to Home, Movies and Series. Escape is not bound: it already clears the search field, closes menus and dialogs, and exits fullscreen in the Player. All four keys appear in the keyboard shortcuts list.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [ ] `⌘[` and the mouse back button pop one detail page, or clear Search when there is none
- [ ] `⌘1`, `⌘2` and `⌘3` select the tabs and close any open detail page, the same as clicking a tab
- [ ] The keys don't fire while typing in a text field, and don't affect the Player
- [ ] The `?` shortcuts list shows Back, Home, Movies and Series under App
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
