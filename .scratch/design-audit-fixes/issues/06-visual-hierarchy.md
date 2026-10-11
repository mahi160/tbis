# 06 — Visual hierarchy: type sizes, section headings, row spacing

**What to build:** Text hierarchy reads clearly without adding new steps to the design system. The three type sizes become meta 11, body 13 and title 20; today meta and body are only 1px apart. On detail pages and Search, section headings (Cast, Episodes, Movies, Series, Recent searches) use body semibold, so the item's name is the largest text on the page. Home row headings and the Library page title keep the title size. Home rows get 24px between them, clearly more than the 8px between a heading and its cards.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [ ] Type sizes are 11 / 13 / 20, and controls, the title bar and settings rows still fit their text
- [x] On Movie and Series detail, the item name is larger than any section heading
- [x] Search's section headings and Recent searches use body semibold
- [x] Home rows are separated by 24px
- [ ] Every screen checked in Raptor Night and Fossil Bone for clipping
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
