# 16 — Light theme: Fossil Slate

**What to build:** The light theme drops the generic cream-and-rust look. It becomes "Fossil Slate": a cool, low-chroma slate-grey page, near-white surfaces and dark slate text, with a terracotta accent. Danger red must stay clearly distinguishable from the terracotta accent. Saved configs that selected "Fossil Bone" open on Fossil Slate automatically. It still sets only the 15 color roles and passes every theme test.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [x] Settings lists Fossil Slate instead of Fossil Bone
- [x] A config saved with "Fossil Bone" loads Fossil Slate
- [x] Neutrals are cool slate (no cream or warm paper tint); no pure white or pure black
- [x] The terracotta accent and danger red are clearly different (hue and/or lightness)
- [x] All theme tests pass (roles only, contrast minimums including accent text at 4.5:1)
- [ ] Screenshots of Home, Movie detail and Settings on Fossil Slate
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
