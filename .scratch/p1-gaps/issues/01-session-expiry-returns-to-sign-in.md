# 01 — Session expiry returns to sign-in

**What to build:** When the Jellyfin server rejects the saved token (HTTP 401) on any request — Home, Movies/Series pages, Search, a detail page, or the Player — tbis clears the saved session and shows the Login screen with a short "Your session expired, sign in again" notice, instead of an error panel reading "HTTP 401". Other HTTP failures keep today's error behaviour.

**Blocked by:** None — can start immediately.

**Status:** done

- [x] A 401 from any authenticated request lands the user on Login with the expiry notice
- [x] The stale session is removed from the saved config, so a relaunch also shows Login
- [x] A 401 during playback closes the Player (mpv torn down) before showing Login
- [x] Non-401 failures (network down, 5xx) still show the existing error panel with Retry
- [x] Covered by a test against a fake server returning 401
