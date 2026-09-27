# 10 — Subtitle styling

**What to build:** A Subtitles section in Settings lets the user set text-subtitle size, colour, and vertical position (and background/outline strength), with a live preview line. Applies to the Player and PiP for text subtitles; image subtitles (PGS/VobSub) keep their own look.

**Blocked by:** p1-gaps 06 — Settings screen with default language.

**Status:** done (runtime unverified)

- [x] Size, colour, position, and outline/box controls in Settings (preset dropdowns), saved to config
- [ ] ~~Changes apply to a currently playing video~~ — N/A: Settings is unreachable while the Player covers the window; styles apply from the next playback
- [x] PiP uses the same styling
- [x] Image-based subtitles are unaffected
