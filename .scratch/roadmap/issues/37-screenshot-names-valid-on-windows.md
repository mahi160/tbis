# 37 — Screenshot names are valid on Windows

**What to build:** A screenshot is named after the item, and only `/` and `:` are replaced. A title containing `?`, `*`, `"`, `<`, `>`, `|` or `\` (for example "What If...?") fails to save on Windows. Replace all of them.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). Screenshot filenames replace every Windows-reserved punctuation character listed in the ticket. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/player/controls.rs` (`fn screenshot`).

## Steps

1. In `fn screenshot`, replace:

   ```rust
   let file = format!("{name} {at}.png").replace(['/', ':'], "-");
   ```

   with:

   ```rust
   // not allowed in a Windows file name; `/` and `:` also break macOS
   let file =
       format!("{name} {at}.png").replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-");
   ```

   (In the source file the backslash character literal is written `'\\'`.) Let `cargo fmt` lay the line out.

## Done when

- [ ] A screenshot of an item whose name contains `?` is saved on macOS with `-` in place of `?`

**Check:** the gate in `PLAN.md`.
