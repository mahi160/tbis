# 03 — Ambient backdrop on Home, Library and Search

**What to build:** Browsing gets an ambient backdrop, like Jellyfin web's backdrops setting but quieter. On Home, Library and Search, a heavily blurred, theme-tinted backdrop of the poster or card under the pointer sits behind the page. It cross-fades as the pointer moves between items, with a short delay so sweeping across a row doesn't thrash. It keeps the last item when the pointer leaves a card, and clears when switching tabs. Home starts with its first item's backdrop. Because text sits directly on the page here (no cards), the tint is stronger than on detail pages, so text stays readable over any image. That is covered by the same contrast test with its own token. Reduced motion swaps instantly. The title bar stays solid, and detail pages, Settings, Watch stats and the Player don't show it.

**Blocked by:** 01 — Frosted, immersive Movie and Episode detail (shared backdrop layer and tokens)

**Status:** done (visual check pending)

- [ ] Hovering a poster or card on Home, Library or Search fades the page backdrop to that item's blurred backdrop (blurred poster when there is none)
- [ ] Quick sweeps across a row don't flicker; the backdrop settles on the item where the pointer stops
- [ ] Leaving a card keeps the current backdrop; switching tabs clears it; Home opens with its first item's backdrop
- [x] Muted text on the browse page stays ≥ 4.5:1 over pure white and pure black images in every theme (test)
- [ ] With reduced motion, the backdrop swaps without a fade
- [ ] Detail pages, Settings, Watch stats and the Player are unaffected; the title bar stays solid
- [ ] Library scrolling stays smooth (the backdrop layer doesn't re-render the grid)
- [ ] Screenshots of Home and Library with an ambient backdrop in Raptor Night and Fossil Slate
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
