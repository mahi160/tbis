# 07 — Autoplay

**What to build:** When an Episode reaches its end, it is marked played and the Player shows "Next: S2E4 · Name" with a 5-second countdown, a Play now button, and a Cancel button. When the countdown ends, or the user presses Play now, the next Episode of the same Series loads in the same Player. Cancel returns to the previous screen. There is no Autoplay after a Movie or after the last Episode of a Series; those return as usual.

**Blocked by:** 05 — Series page and Series detail

**Status:** ready-for-agent

- [ ] The countdown appears at the end of an Episode
- [ ] Play now and the finished countdown both start the next Episode, crossing season boundaries
- [ ] Cancel returns without playing
- [ ] The last Episode and any Movie return with no countdown
