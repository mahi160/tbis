# 02 — Skip Intro / Skip Credits

**What to build:** While an Episode or Movie plays, the Player knows the item's Intro and Outro (credits) media segments from the server (Jellyfin 10.10+ media segments). While playback is inside one, a "Skip Intro" / "Skip Credits" button shows over the video; clicking it or pressing `S` seeks to the segment's end. Skipping credits on an Episode with a next Episode behaves like Autoplay's "Play now".

**Blocked by:** None — can start immediately.

**Status:** done

- [x] Segments are fetched once per loaded item; a server without segment support just shows no button (no error)
- [x] The button appears only while the playhead is inside a segment and hides when it leaves
- [x] `S` skips the current segment; does nothing outside one
- [x] Skip Credits on an Episode with a next Episode starts it (counts current as played); otherwise it seeks to the segment end
- [x] Works after Autoplay moves to the next Episode (segments reset per item)
- [x] Button stays visible while in a segment (like the up-next card, not auto-hidden) and works in fullscreen
