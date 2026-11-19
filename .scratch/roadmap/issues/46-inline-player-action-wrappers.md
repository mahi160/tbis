# 46 — Inline the Player's action wrappers

**What to build:** `src/player/controls.rs` has 15 methods that exist only so a key action can call another method with fixed arguments (`seek_back` calls `seek_step(false, false, ...)`). `src/player/pip.rs` already registers its actions as closures instead. Do the same here and delete the 15 wrappers. Behaviour does not change.

**Model:** haiku

**Blocked by:** 33 and 34 (they edit the same `render`; do them first).

**Status:** done (runtime unverified). Removed the fifteen PlayerView action wrappers and updated key, chrome and media-key callers. PipView::toggle_pause is unchanged. Closure formatting may offset the ticket's illustrative line-count reduction. After formatting, controls.rs is 926 lines versus 987 before this cleanup (61 fewer), not the requested 90; all fifteen wrappers are removed. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/player/controls.rs` (all of it), `src/player/controls/chrome.rs` (`fn bottom_chrome`), `src/player/pip.rs` (the `.on_action(...)` lines in `impl Render for PipView`, as the pattern to copy).

## Steps

1. In `impl Render for PlayerView` in `src/player/controls.rs`, replace each `.on_action` line in the left column with the one in the right column. Leave every other `.on_action` line as it is.

   | Old: `.on_action(cx.listener(Self::...))` | New: `.on_action(cx.listener(...))` |
   | --- | --- |
   | `Self::toggle_pause` | `\|this, _: &TogglePause, window, cx\| this.pause_toggle(window, cx)` |
   | `Self::seek_back` | `\|this, _: &SeekBack, window, cx\| this.seek_step(false, false, window, cx)` |
   | `Self::seek_forward` | `\|this, _: &SeekForward, window, cx\| this.seek_step(true, false, window, cx)` |
   | `Self::seek_back_long` | `\|this, _: &SeekBackLong, window, cx\| this.seek_step(false, true, window, cx)` |
   | `Self::seek_forward_long` | `\|this, _: &SeekForwardLong, window, cx\| this.seek_step(true, true, window, cx)` |
   | `Self::toggle_fullscreen` | `\|this, _: &ToggleFullscreen, window, cx\| this.fullscreen_toggle(window, cx)` |
   | `Self::volume_up` | `\|this, _: &VolumeUp, window, cx\| this.nudge_volume(true, window, cx)` |
   | `Self::volume_down` | `\|this, _: &VolumeDown, window, cx\| this.nudge_volume(false, window, cx)` |
   | `Self::speed_up` | `\|this, _: &SpeedUp, window, cx\| this.step_speed(1, window, cx)` |
   | `Self::speed_down` | `\|this, _: &SpeedDown, window, cx\| this.step_speed(-1, window, cx)` |
   | `Self::sub_delay_later` | `\|this, _: &SubDelayLater, window, cx\| this.adjust_delay(true, SUB_DELAY_STEP, window, cx)` |
   | `Self::sub_delay_earlier` | `\|this, _: &SubDelayEarlier, window, cx\| this.adjust_delay(true, -SUB_DELAY_STEP, window, cx)` |
   | `Self::audio_delay_later` | `\|this, _: &AudioDelayLater, window, cx\| this.adjust_delay(false, AUDIO_DELAY_STEP, window, cx)` |
   | `Self::audio_delay_earlier` | `\|this, _: &AudioDelayEarlier, window, cx\| this.adjust_delay(false, -AUDIO_DELAY_STEP, window, cx)` |
   | `Self::skip_segment_action` | `\|this, _: &SkipSegment, window, cx\| this.skip_segment(window, cx)` |

   (The `\|` in the table is a plain `|` in code.)

2. Delete these 15 methods from `impl PlayerView` in `src/player/controls.rs`: `toggle_pause`, `seek_back`, `seek_forward`, `seek_back_long`, `seek_forward_long`, `toggle_fullscreen`, `volume_up`, `volume_down`, `speed_up`, `speed_down`, `sub_delay_later`, `sub_delay_earlier`, `audio_delay_later`, `audio_delay_earlier`, `skip_segment_action`. Move the doc comment that sits on `volume_up` ("Actual `self.volume` update comes back through ...") onto `fn nudge_volume`.

   Do NOT delete or rename `PipView::toggle_pause` in `src/player/pip.rs`; it is a different type.

3. Fix the remaining callers of the deleted methods:
   - `fn on_remote` in `src/player/controls.rs`: `self.toggle_pause(&TogglePause, window, cx)` becomes `self.pause_toggle(window, cx)`; `self.seek_forward(&SeekForward, window, cx)` becomes `self.seek_step(true, false, window, cx)`; `self.seek_back(&SeekBack, window, cx)` becomes `self.seek_step(false, false, window, cx)`.
   - `fn bottom_chrome` in `src/player/controls/chrome.rs`: `this.toggle_pause(&TogglePause, window, cx)` becomes `this.pause_toggle(window, cx)`; `this.toggle_fullscreen(&ToggleFullscreen, window, cx)` becomes `this.fullscreen_toggle(window, cx)`.

4. Run `cargo clippy --all-targets -- -D warnings` and remove any import it reports as unused. Do not remove an import it does not report.

## Done when

- [x] `rg -n "fn (seek_back|seek_forward|seek_back_long|seek_forward_long|volume_up|volume_down|speed_up|speed_down|sub_delay_later|sub_delay_earlier|audio_delay_later|audio_delay_earlier|skip_segment_action|toggle_fullscreen)\b" src/player` prints nothing
- [ ] `wc -l src/player/controls.rs` is at least 90 lines shorter than before
- [ ] Every Player key still works: space, arrows, alt-arrows, f, up/down, `<` `>`, `[` `]`, shift-`[` shift-`]`, s; and the media keys

**Check:** the gate in `PLAN.md`.
