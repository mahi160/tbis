# 15 — Auto-skip intros and credits

**What to build:** Skipping an intro or credits takes a click or the `s` key. Add a Settings switch, off by default, that skips them automatically when playback enters the segment.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). `auto_skipped` is a field of `Playback` (the per-load state), so `Playback::new` resets it. The switch is in the Seeking group. Ticket 17 changed its default to on.

**Read first:** `src/player/playback.rs` (`fn active_segment`, `fn skip_segment`, `fn on_mpv` `TimePos` arm, the `segments` field), `src/config.rs` (`HideSpoilers` is the pattern: a newtype bool that is also an app global), `src/settings.rs` (how `hide_spoilers` is wired), `src/app.rs` (`AppView::new` sets globals; `fn open_settings` updates them).

## Steps

1. In `src/config.rs`, add `AutoSkip(pub bool)` copied from `HideSpoilers` (same derives, `#[serde(transparent)]`, `impl gpui_kit::Global`), with the doc comment "Skip intros and credits without asking (Settings).". Add the `Config` field `auto_skip` with `#[serde(default)]` and the entry in `fn defaults`.
2. In `src/app.rs`, `AppView::new`, add `cx.set_global(config.auto_skip);` beside the `hide_spoilers` global.
3. Wire Settings exactly like `hide_spoilers`: `SettingsChanged::AutoSkip(AutoSkip)`, an entity, an observer (the `_observe` array grows by one), and a switch item "Skip intros and credits automatically" in the Playback page. In `fn open_settings`, the new arm sets `this.config.auto_skip` and `cx.set_global(...)`.
4. In the Player, add a field `auto_skipped: Option<f64>` (the `start` of the last segment skipped automatically). Reset it to `None` where a new item loads.
5. In `fn on_mpv`, `TimePos` arm, after the time is stored:

   ```rust
   if cx.global::<AutoSkip>().0
       && let Some(segment) = self.active_segment()
       && self.auto_skipped != Some(segment.start)
   {
       self.auto_skipped = Some(segment.start);
       self.skip_segment(window, cx);
   }
   ```

   Recording the segment start means seeking back into an intro does not skip it a second time.

## Done when

- [ ] The switch is in Settings > Playback, off by default, saved to `config.json`
- [ ] With it on, an intro is skipped once when playback enters it
- [ ] Seeking back into the same intro leaves it playing, with the Skip button still offered
- [ ] With it off, behaviour is unchanged

**Check:** the gate in `PLAN.md`, then play an Episode that has intro markers with the switch on.
