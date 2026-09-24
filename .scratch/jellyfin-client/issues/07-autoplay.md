# 07 — Autoplay

**What to build:** In the last 30 seconds of an Episode, a card in the bottom-right corner of the Player shows "Next: S2E4 · Name", a countdown of the playback time left, a Play now button, and a Cancel button. The countdown follows the video, so pausing pauses it and seeking back before the last 30 seconds hides the card. When the Episode ends, or the user presses Play now (or Enter), the current Episode is marked played and the next Episode of the same Series, in the server's episode order, loads in the same Player. Cancel (or Esc) hides the card; the Episode keeps playing and the Player returns to the previous screen when it ends. There is no Autoplay after a Movie or after the last Episode of a Series; those return as usual.

**Blocked by:** 05 — Series page and Series detail

**Status:** done

- [ ] The card appears in the last 30 seconds of an Episode and counts down with playback
- [ ] Play now, Enter, and the end of the Episode all start the next Episode, crossing season boundaries
- [ ] Cancel and Esc hide the card, and the Player returns without autoplay when the Episode ends
- [ ] The last Episode and any Movie return with no countdown
