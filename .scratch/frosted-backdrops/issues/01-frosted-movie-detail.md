# 01 — Frosted, immersive Movie and Episode detail

**What to build:** Movie and Episode detail become immersive, in the spirit of Jellyfin web's backdrops but with a modern frosted look. The item's backdrop fills the whole page behind the content. It is heavily blurred (blurred by the server, since gpui can't blur) and tinted with the theme's background color. It stays fixed while the page scrolls. A faint, sharp copy of the backdrop shows only at the top and fades into the blurred field. The content sits in frosted cards: translucent theme surface with a hairline border, light frost on Fossil Slate and dark frost on the dark themes. There is one card for the header (art, title, meta, actions) and one for cast. An Episode uses its Series backdrop, never its own still. With no backdrop, the poster is blurred instead. The tint and card strengths are design tokens. A test proves text stays readable over any image: muted text reaches at least 4.5:1 and body text at least 7:1 over pure white and over pure black images, in every theme. This replaces the current faint 30% backdrop.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [ ] Movie and Episode detail show a full-page blurred, theme-tinted backdrop that stays put while the content scrolls
- [ ] A faint sharp backdrop shows at the top and fades into the blurred field with no hard edge
- [ ] Header and cast sit in frosted cards that follow the theme (dark frost on dark themes, light frost on Fossil Slate)
- [ ] An Episode shows its Series backdrop; an item without a backdrop uses its blurred poster; an item with neither shows the plain theme background
- [x] Tint and card strengths are design tokens; a test checks muted text ≥ 4.5:1 and body text ≥ 7:1 over pure white and pure black images for every theme
- [x] The old faint backdrop component is removed if nothing else uses it (Series still uses `components/backdrop.rs`; ticket 02 deletes it)
- [ ] Screenshots of Movie and Episode detail in Raptor Night and Fossil Slate
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
