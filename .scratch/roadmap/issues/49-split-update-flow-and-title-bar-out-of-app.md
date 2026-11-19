# 49 — Move the update flow and the title bar out of `app.rs`

**What to build:** `src/app.rs` is over 1000 lines and mixes screen navigation with two things that have nothing to do with it: the update check/offer/install flow and the title bar. Move each into its own file as a second `impl AppView` block, the way `src/player/` splits `PlayerView` across `playback.rs` and `controls.rs`. Code moves; no line of logic changes.

**Model:** haiku

**Blocked by:** 41 (it edits `install_update`; do it first so the moved code is final).

**Status:** done (runtime unverified). Moved update methods and title-bar rendering into src/app/updates.rs and src/app/title_bar.rs. Method bodies are unchanged from the preceding fixes; src/app.rs is below 900 lines. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/app.rs` (`fn check_for_updates`, `fn offer_update`, `fn install_update`, `fn render_title_bar`, and the `use` lines at the top), `src/player/mod.rs` (the `mod` lines at the top) and `src/player/playback.rs` (its first 20 lines), as the pattern.

## Steps

1. Create `src/app/updates.rs` starting with:

   ```rust
   //! Update check, offer and install for `AppView`.

   use super::*;

   impl AppView {
   }
   ```

   Cut `check_for_updates`, `offer_update` and `install_update` (with their doc comments) out of `impl AppView` in `src/app.rs` and paste them inside this `impl`. Keep `check_for_updates` `pub`. Do not change their bodies.

2. Create `src/app/title_bar.rs` the same way (`//! Title bar for \`AppView\`.`, `use super::*;`, an `impl AppView` block) and move `render_title_bar` into it, changing `fn render_title_bar` to `pub(super) fn render_title_bar`.

3. In `src/app.rs`, after the `use` lines, add:

   ```rust
   mod title_bar;
   mod updates;
   ```

4. Run `cargo clippy --all-targets -- -D warnings`.
   - For an "unused import" warning in `src/app.rs`: the import is now needed only in a moved file. Move that `use` line (or just the unused name) from `src/app.rs` into the moved file that uses it. `use super::*;` does not re-export `app.rs`'s private imports of names it no longer uses.
   - For a "cannot find" error in a moved file: add the matching `use` line there, copied from the top of `src/app.rs`.
   - For a "private" error on a helper function or type defined in `src/app.rs`: it is visible to child modules already; the name is missing an import, not visibility. If a moved function calls a method that stayed private in another `impl` block of a sibling file, make that method `pub(super)`.

Do not rename anything and do not change any function body.

## Done when

- [x] `wc -l src/app.rs` is under 900
- [ ] `git diff --stat` shows only `src/app.rs`, `src/app/updates.rs` and `src/app/title_bar.rs`
- [ ] The title bar, the user menu, search, and "Check for updates" all work as before

**Check:** the gate in `PLAN.md`.
