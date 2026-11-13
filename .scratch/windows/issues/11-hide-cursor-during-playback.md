# 11 — Hide the cursor during playback

**What to build:** `platform::windows::cursor::hide` (or gpui's cursor API, if it covers Windows) hides the pointer when the Player's controls fade, and it comes back on mouse move like on macOS.

**Blocked by:** 06.

**Status:** needs-human (code landed; manual checks pending)

- [ ] The cursor hides with the controls and returns on movement
- [ ] It never stays hidden after leaving the Player
