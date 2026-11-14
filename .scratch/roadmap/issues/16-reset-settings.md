# 16 — Reset settings

**What to build:** Settings has no way back to the defaults. Add "Reset all settings" to the Settings screen. It restores every value shown on the Settings pages and keeps the sign-in, the device identity, the window size, search history, volume and per-item track memory. `mpv.conf` is left alone.

**Model:** haiku

**Blocked by:** 09 and 15 when they are done (their new entities must be reset too; include them if present).

**Status:** done (runtime unverified). The reset also covers `video_quality` (09) and `auto_skip` (15), using a small local `set` helper in place of the repeated `update` closures.

**Read first:** `src/settings.rs` (all of `SettingsView::new`, `fn appearance_page`, `fn render`), `src/components/settings.rs` (`SettingField`, `fn render`), `src/config.rs` (the `Default` impls), `src/theme.rs` (`NAMES`; the first is the default), `src/components/button.rs` (`outline_button`).

## Steps

1. In `src/components/settings.rs`, add a button field. In `enum SettingField` add `Action(SharedString, Rc<dyn Fn(&mut App)>)`, the constructor:

   ```rust
   pub fn action(label: impl Into<SharedString>, run: impl Fn(&mut App) + 'static) -> Self {
       Self::Action(label.into(), Rc::new(run))
   }
   ```

   and the render arm:

   ```rust
   Self::Action(label, run) => {
       let run = run.clone();
       outline_button(id, label.clone(), cx)
           .on_click(move |_, _, cx| run(cx))
           .into_any_element()
   }
   ```

   Import `outline_button` from `super::button`.
2. In `src/settings.rs`, add fields `confirm_reset: bool` and `_confirm_reset: Task<()>` to `SettingsView` (initial `false` and `Task::ready(())`).
3. Add the reset. Every entity already has an observer that emits `SettingsChanged`, so setting the entity and notifying is enough for the app to save:

   ```rust
   /// Restores every setting on these pages to its default; observers save each one.
   fn reset(&mut self, cx: &mut Context<Self>) {
       self.language.update(cx, |v, cx| { *v = LanguagePref::default(); cx.notify(); });
       self.subtitles.update(cx, |v, cx| { *v = SubtitleStyle::default(); cx.notify(); });
       self.max_bitrate.update(cx, |v, cx| { *v = None; cx.notify(); });
       self.shaders.update(cx, |v, cx| { *v = ShaderProfile::default(); cx.notify(); });
       self.seek.update(cx, |v, cx| { *v = SeekSteps::default(); cx.notify(); });
       self.theme.update(cx, |v, cx| { *v = theme::NAMES[0].into(); cx.notify(); });
       self.hide_spoilers.update(cx, |v, cx| { *v = HideSpoilers::default(); cx.notify(); });
       self.backdrops.update(cx, |v, cx| { *v = ShowBackdrops::default(); cx.notify(); });
       self.audio.update(cx, |v, cx| { *v = AudioStyle::default(); cx.notify(); });
   }
   ```

   Add one line for each entity added by tickets 09 and 15 when they exist. Let `cargo fmt` lay the lines out.
4. Add the two-step button handler:

   ```rust
   fn reset_clicked(&mut self, cx: &mut Context<Self>) {
       if self.confirm_reset {
           self.confirm_reset = false;
           self._confirm_reset = Task::ready(());
           self.reset(cx);
       } else {
           self.confirm_reset = true;
           self._confirm_reset = cx.spawn(async move |this, cx| {
               cx.background_executor().timer(std::time::Duration::from_secs(4)).await;
               this.update(cx, |this, cx| {
                   this.confirm_reset = false;
                   cx.notify();
               })
               .ok();
           });
       }
       cx.notify();
   }
   ```

5. Add a third page built in a new `fn general_page(&self, cx: &Context<Self>) -> SettingPage` named "General" with one group and one item titled "Reset all settings", description "Restores everything on these pages. Your sign-in, window size and mpv.conf are kept.", field `SettingField::action(label, ...)` where the label is "Click again to reset" while `self.confirm_reset` and "Reset" otherwise. The closure calls `reset_clicked` through `cx.entity().downgrade()` and `.update(cx, ...)`.
6. Add the page to the `.pages([...])` list in `fn render`, last.

## Done when

- [ ] Settings shows General > Reset all settings
- [ ] The first click changes the label to "Click again to reset" for four seconds; the second click resets
- [ ] After a reset every control on the Playback and Appearance pages shows its default, the theme switches back, and `config.json` holds the defaults
- [ ] `session`, `device_id`, `window`, `recent_searches`, `volume` and `track_prefs` in `config.json` are unchanged

**Check:** the gate in `PLAN.md`. Runtime: change five settings, reset, and compare `config.json` before and after (`~/Library/Application Support/tbis-dev/config.json` for a `cargo run`).
