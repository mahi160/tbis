# 01 — Player overlays readable on every theme

**What to build:** Text and controls drawn over video in the Player stay readable whatever theme is active. Today the up-next card, the Player error panel and the Playback info panel take some colors from the theme, so on the light theme (Fossil Bone) the Cancel button is almost invisible (1.18:1) and captions, error descriptions and info headings fall to about 3:1. Everything over video uses the fixed overlay tints instead. The shared error panel keeps the theme look on list screens and gets an on-video look for the Player.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [ ] On Fossil Bone, the up-next card's title, countdown caption and Cancel button reach at least 4.5:1 against the card
- [ ] On Fossil Bone, the Player error panel's title and description reach at least 4.5:1
- [ ] On Fossil Bone, the Playback info group headings reach at least 4.5:1, and the panel title is semibold, not bold
- [ ] The error panel on Home and Library looks the same as before
- [ ] Overlays on the three dark themes look the same as before, apart from the info headings, which are now white instead of the accent color
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
