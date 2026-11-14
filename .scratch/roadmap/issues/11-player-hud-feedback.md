# 11 — On-screen feedback for seek, speed and mute

**What to build:** Pressing a seek, speed or mute key gives no visible confirmation. The Player already shows a short text banner for subtitle and audio sync (`sync_hud`). Turn that into one general banner and use it for seek ("+10 s", "−60 s"), speed ("1.25×") and mute ("Muted", "Unmuted").

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified)

**Read first:** `src/player/controls.rs` (`fn adjust_delay`, `fn seek_step`, `fn step_speed`, `fn speed_reset`, `fn toggle_mute`, the nudge and frame functions, `fn render`), `src/player/mod.rs` (fields `sync_hud`, `_sync_hud`, const `VOLUME_HUD_AFTER`), `src/player/playback.rs` (where `sync_hud` is cleared).

## Steps

1. Rename the fields `sync_hud` to `hud` and `_sync_hud` to `_hud` everywhere (`rg -n "sync_hud" src`).
2. In `src/player/controls.rs`, add:

   ```rust
   /// Shows `message` in the banner for a moment; a newer message replaces it.
   fn flash(&mut self, message: String, window: &mut Window, cx: &mut Context<Self>) {
       self.hud = Some(message.into());
       self._hud = cx.spawn_in(window, async move |this, cx| {
           cx.background_executor()
               .timer(super::VOLUME_HUD_AFTER)
               .await;
           this.update(cx, |this, cx| {
               this.hud = None;
               cx.notify();
           })
           .ok();
       });
       cx.notify();
   }
   ```

3. In `fn adjust_delay`, replace the inline banner code (the `self.sync_hud = ...` assignment and the spawned task) with `self.flash(format!("{label} sync: {delay:+.1} s"), window, cx);`.
4. Call `flash` from:
   - `fn seek_step`: `let steps = *cx.global::<SeekSteps>();`, pick `steps.long` or `steps.short`, then `format!("{}{} s", if forward { "+" } else { "\u{2212}" }, seconds)`.
   - `fn step_speed` and `fn speed_reset`: `format!("{}\u{d7}", speed)` with the speed just set.
   - `fn toggle_mute`: `"Unmuted"` when `self.muted` was true, else `"Muted"`.
5. Leave `seek_by_step` unchanged: PiP calls it without a window.

## Done when

- [ ] Arrow keys show "+10 s" or "−10 s" (the configured step), Option+arrows the long step
- [ ] `<`, `>` and Backspace show the new speed
- [ ] `m` shows Muted or Unmuted
- [ ] Subtitle and audio sync banners still work
- [x] A second key press within the display time replaces the banner text

**Check:** the gate in `PLAN.md`, then play a video and press each key.
