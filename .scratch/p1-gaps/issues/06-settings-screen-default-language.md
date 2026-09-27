# 06 — Settings screen with default language

**What to build:** A Settings screen opened from the title-bar user menu, with a minimal layout that later settings (subtitle styling, etc.) slot into. Its first real setting is preferred audio language and preferred subtitle language (including "off"). When an item is played and no remembered track pick applies, the Player chooses audio and subtitle tracks by these preferences. Settings persist across launches.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] User menu has "Settings", which opens the Settings screen; back/tab navigation leaves it
- [x] Preferred audio language and preferred subtitle language (with "Off") can be picked and are saved to config
- [x] Remembered track picks still win over the language preference
- [x] With no remembered pick, playback starts on the preferred-language tracks when the item has them, else server defaults
- [x] Settings screen structure makes adding the next setting a one-section change
