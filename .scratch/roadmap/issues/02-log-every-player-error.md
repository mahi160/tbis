# 02 — Log every Player error

**What to build:** When playback fails, the reason is shown on screen but not always written to the log, so a Windows user's `tbis.log` can be empty about the failure. Write every Player error to the log at the moment it is set.

**Model:** haiku

**Blocked by:** None.

**Status:** done

**Read first:** `src/player/mod.rs` (`PlayerView::new`, `fn start`), `src/player/playback.rs` (`fn on_mpv`, `fn on_player_event`), `src/log.rs` (`pub fn write`).

## Steps

1. In `src/player/playback.rs`, add a method on `PlayerView` next to `update_time`:

   ```rust
   /// Shows `message` in the error panel and records it in the log.
   pub(super) fn fail(&mut self, message: String) {
       crate::log::write(&format!("player error: {message}"));
       self.playback.error = Some(message.into());
   }
   ```

2. Replace each of these assignments with a call to `self.fail(...)`, keeping the same message text:
   - `src/player/mod.rs`, `fn start`: `Cannot load item: {err}` and the `Err(err) => self.playback.error = Some(err.into())` arm after `mpv.load` (pass `err` as the message).
   - `src/player/playback.rs`, `fn on_mpv`: `Playback failed: {err}` in the `FileEvent::EndFile(Err(err))` arm.
   - `src/player/playback.rs`, `fn on_player_event`: `Playback failed: {err}` in the `PlayerEvent::RenderFailed` arm.
3. In `src/player/mod.rs`, `PlayerView::new`, the `Cannot start player: {err}` error is built before `self` exists. Log it there directly: build the message into a `let message = format!(...)`, call `crate::log::write(&format!("player error: {message}"))`, then use `message.into()`.
4. Run `rg -n "playback.error = Some" src/player`. Every remaining hit must be inside `fn fail` or the `mpv_error` assignment in `PlayerView::new`.

## Done when

- [x] All five error sites write a `player error:` line
- [x] Message texts on screen are unchanged

**Check:** the gate in `PLAN.md`.
