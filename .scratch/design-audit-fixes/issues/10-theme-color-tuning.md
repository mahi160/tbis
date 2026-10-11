# 10 — Theme color tuning and quieter watched badge

**What to build:** Themes support the artwork instead of competing with it:

- Plesio Lagoon's background and surface colors are about half as saturated (chroma around 0.035 instead of 0.075). The cyan accent stays.
- Stego Moss's accent has more chroma and is shifted about 20° in hue away from its neutrals, so selected, hover and primary states separate.
- Fossil Bone keeps its warm palette, but its surface is off-white instead of pure white.

The contrast test for accent-colored text rises from 3:1 to 4.5:1, because titles turn the accent color on hover. The watched check on posters and Episode stills becomes a dark tint with a white check instead of an accent-colored badge, so grids of watched items get quieter.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [x] The theme contrast tests pass with the accent-on-background minimum raised to 4.5:1
- [ ] Plesio Lagoon's background is visibly less saturated, and posters read cleanly on it
- [ ] Stego Moss's primary buttons and selected tab differ clearly from hover fills
- [x] No theme uses pure white as a surface
- [x] The watched check is dark and white in every theme
- [ ] Screenshots of Home and Library in all four themes
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
