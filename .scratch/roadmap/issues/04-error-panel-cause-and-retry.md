# 04 — Error panel shows the cause and offers Retry

**What to build:** Every failure is titled "Can't play this", and the only button is "Go back". Give the panel a title that says what failed, and a Retry button when retrying can work.

**Model:** haiku

**Blocked by:** 02 (uses `fn fail`).

**Status:** done (runtime unverified). Retry calls `reload_current`, extracted from `set_max_bitrate`: it ends the old session (Stopped report, `mpv.stop`), builds a fresh `Playback`, and resumes from `playback.time`, or `start_at` when nothing played yet. No separate `retry` method.

**Read first:** `src/player/controls.rs` (`fn render`, the `let error = ...` block), `src/player/playback.rs` (`fn load`, `fn fail`, `fn set_max_bitrate`), `src/components/status.rs` (`error_panel`).

## Steps

1. In `src/player/controls.rs`, `fn render`, compute the title from the message prefix before building the panel:

   ```rust
   let title = if error.starts_with("Cannot start player") {
       "The player could not start"
   } else if error.starts_with("Cannot load item") {
       "This item could not be loaded"
   } else {
       "Playback stopped"
   };
   ```

   Pass `title` to `error_panel` instead of `"Can't play this"`.
2. Add a method in `src/player/playback.rs`:

   ```rust
   /// Reloads the current item from where it stopped, after a load or playback failure.
   pub(super) fn retry(&mut self, cx: &mut Context<Self>) {
       self.playback.error = None;
       let id = self.playback.source.id.clone();
       self.load(id, Some(self.playback.time), cx);
   }
   ```

   Read `fn load` first and match its real parameter types; `set_max_bitrate` in the same file reloads the current item and is the pattern to copy.
3. In the `let error = ...` block, build a second button only when `self.mpv.is_some()`:

   ```rust
   let retry = button("error-retry", "Retry")
       .on_click(cx.listener(|this, _, _, cx| this.retry(cx)))
       .into_any_element();
   ```

   Pass `[retry, back]` when mpv exists and `[back]` otherwise. `error_panel` takes an iterator of buttons; build a `Vec<AnyElement>` for both cases.

## Done when

- [x] The three titles appear for the three message prefixes
- [x] Retry is shown only while `self.mpv` exists, and it reloads from the stopped position
- [x] "Go back" still closes the Player

**Check:** the gate in `PLAN.md`. To see the panel, start a video and stop the Jellyfin server or disconnect the network.
