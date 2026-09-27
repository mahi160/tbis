# 03 — Mark watched / unwatched and Favorite

**What to build:** Movie detail, Series detail, and each Episode row on Series detail get a watched toggle and a Favorite toggle that update the server immediately. Marking a Series watched marks every Episode played. Home rows and the Movies/Series pages show the new state the next time they refresh (e.g. a watched Movie leaves the Movies row, an unwatched Episode can reappear in Next Up).

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] Watched toggle on Movie detail, Series detail, and every Episode row
- [x] Favorite toggle on Movie detail and Series detail
- [x] Toggle updates optimistically and reverts with an inline error if the server call fails (Series watched also reloads its Episodes)
- [x] Returning to Home or a Library page reflects the change without an app restart
- [x] Clearing watched also clears the saved resume position, matching Jellyfin web
