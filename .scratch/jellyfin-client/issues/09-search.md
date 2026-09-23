# 09 — Search

**What to build:** The title bar has a search field, and ⌘F focuses it. Typing searches the server for Movies, Series, and Episodes across all Libraries, and the results appear on a Search screen grouped by type. Clicking a Movie or an Episode plays it in the Player. Clicking a Series opens Series detail. Clearing the field, or pressing Esc, returns to the previous tab.

**Blocked by:** 03 — Player tracer bullet. Series results stay non-clickable until 05 — Series page and Series detail lands.

**Status:** done

- [ ] ⌘F focuses the title-bar search field from any tab
- [ ] Results cover Movies, Series, and Episodes, grouped by type
- [ ] Typing quickly does not flood the server, and stale results never replace newer ones
- [ ] Clicking a Movie or an Episode plays it
- [ ] Clicking a Series opens Series detail once ticket 05 exists
- [ ] Esc, or an empty field, returns to the previous tab
- [ ] "No results" is shown when nothing matches
