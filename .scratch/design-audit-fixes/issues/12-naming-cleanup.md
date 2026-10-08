# 12 — Doc-comment naming cleanup

**What to build:** Doc comments describe what the code actually draws. Buttons, the watched check and avatars use the one shared corner radius, but their docs call them "round". The star rating is documented as warning-colored, but warning maps to the theme's primary color. Fix these comments. Function renames are out of scope unless trivial. There is no visual change.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [x] No doc comment calls a shared-radius element round or circular
- [x] The star rating's doc names the color it actually uses
- [x] No behaviour or visual change
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
