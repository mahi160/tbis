# 47 — Remove pass-through helpers

**What to build:** Three helpers only forward to something else, and two components write `0.5` where the token `opacity::HALF` exists. Remove the indirection. Behaviour does not change.

**Model:** haiku

**Blocked by:** 38 (it edits the same lines of `fn report`) and 46 (it deletes many `active_mpv` call sites first).

**Status:** done (runtime unverified). Removed active_mpv, Api::post and Api::trickplay pass-throughs; callers use the underlying APIs directly. Disabled button and hero opacity use opacity::HALF. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/player/mod.rs` (`fn active_mpv`), `src/jellyfin/api.rs` (`fn post`, `fn request`, `pub fn trickplay`, `pub async fn report`), `src/jellyfin/types.rs` (`PlaybackItem::trickplay`), `src/player/controls/chrome.rs` (`fn trickplay_preview`), `src/components/button.rs` (`fn ghost_disabled`), `src/components/hero.rs` (the `let (lit, unlit) = ...` lines), `src/design.rs` (`pub mod opacity`).

## Steps

1. **`active_mpv`.** It returns `self.mpv.as_ref()` and nothing else. In `src/player/`, replace every `self.active_mpv()` with `self.mpv.as_ref()` and every `this.active_mpv()` with `this.mpv.as_ref()`. Then delete `fn active_mpv` from `src/player/mod.rs`. Check: `rg -n "active_mpv" src` prints nothing.

2. **`Api::trickplay`.** In `src/jellyfin/types.rs`, change `pub(super) fn trickplay(&self)` on `PlaybackItem` to `pub fn trickplay(&self)`. In `fn trickplay_preview` in `src/player/controls/chrome.rs`, replace:

   ```rust
   let info = self
       .api
       .trickplay(item)
       .filter(|i| i.width > 0 && i.height > 0)?;
   ```

   with:

   ```rust
   let info = item
       .trickplay()
       .filter(|i| i.width > 0 && i.height > 0)?;
   ```

   Delete `pub fn trickplay` and its doc comment from `impl Api` in `src/jellyfin/api.rs`.

3. **`Api::post`.** It has one caller. In `pub async fn report`, change `self.post(path, body).await` to `self.request(Method::POST, path, body).await`. Delete `async fn post` from `impl Api`.

4. **`opacity::HALF`.** In `src/components/button.rs` (`fn ghost_disabled`) and `src/components/hero.rs` (`let (lit, unlit) = ...`), change `.opacity(0.5)` to `.opacity(opacity::HALF)`. Add `opacity` to each file's existing `use crate::design::{...};` import.

5. Run `cargo clippy --all-targets -- -D warnings` and remove any import it reports as unused.

## Done when

- [x] `rg -n "active_mpv|fn post\b|\.opacity\(0\.5\)" src` prints nothing
- [x] `rg -n "pub fn trickplay\b" src/jellyfin/api.rs` prints nothing
- [ ] The timeline hover thumbnail still shows

**Check:** the gate in `PLAN.md`.
