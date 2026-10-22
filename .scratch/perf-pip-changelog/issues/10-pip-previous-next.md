# 10 — PiP Previous/Next Episode buttons

**What to build:** When an Episode plays in Picture-in-Picture, show Previous Episode and Next Episode buttons beside play/pause, with the same behaviour as the Player's (switch without counting the current Episode watched). Each button is hidden when there is no such Episode; Movies show neither. Switching keeps the video in PiP and the PiP window keeps the video's aspect ratio. The Player's Previous/Next Episode keys also work in PiP.

**Blocked by:** 01 — Time updates redraw only the timeline (both change how PiP mirrors Player state).

**Status:** done (runtime unverified). The window keeps the first Episode's aspect lock.

- [ ] Playing an Episode in PiP shows Previous/Next buttons on hover, each only when that Episode exists
- [ ] Clicking one loads that Episode, and playback continues in PiP without returning to the Player
- [ ] Movies show no Previous/Next buttons in PiP
- [ ] The Previous/Next Episode shortcuts work while PiP has focus
- [ ] Buttons are built through the shared button component, with no hardcoded sizes or colors
