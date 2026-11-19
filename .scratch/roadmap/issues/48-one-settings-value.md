# 48 — One Settings value instead of thirteen events

**What to build:** Every setting is written out in three places: its own `Entity` and observer in `SettingsView` plus a `SettingsChanged` variant, an arm of a 13-arm match in `AppView::open_settings`, and a line in `SettingsView::reset`. Adding a setting means editing all three, and forgetting one is silent. Hold the settings in one struct, emit one event, and reset with `Settings::default()`. Nothing the user sees changes, and `config.json` keeps its exact shape.

**Model:** sonnet (one design is fixed below; the edits inside `settings.rs` need judgement)

**Blocked by:** None.

**Status:** done (runtime unverified). Settings now holds all thirteen editable values, with one entity, observer and event. Theme/backdrop/layout/grouping side effects remain conditional. Tests cover all-value round trips, serialization compatibility and reset preserving unrelated fields. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/settings.rs` (all of it), `src/app.rs` (`fn open_settings`, `AppView::new` where the globals are first set), `src/config.rs` (`struct Config`, the `Default` impls of the setting types, `fn defaults`), `src/theme.rs` (`fn active`, `fn apply`, `NAMES`).

## Design (do not change it)

1. In `src/settings.rs`, add a plain struct that holds the thirteen values `SettingsView` edits today, with the same types its entities hold:

   ```rust
   /// Everything the Settings pages edit, as one value.
   #[derive(Clone, PartialEq)]
   pub struct Settings {
       pub language: LanguagePref,
       pub subtitles: SubtitleStyle,
       pub max_bitrate_mbps: Option<u32>,
       pub shaders: ShaderProfile,
       pub video_quality: VideoQuality,
       pub seek: SeekSteps,
       pub theme: String,
       pub hide_spoilers: HideSpoilers,
       pub backdrops: ShowBackdrops,
       pub home_layout: HomeLayout,
       pub library_grouping: LibraryGrouping,
       pub audio: AudioStyle,
       pub auto_skip: AutoSkip,
   }
   ```

   with `Settings::from_config(config: &Config) -> Self` (theme is `theme::active(config).to_string()`), `Settings::write_to(&self, config: &mut Config)` (the thirteen assignments that are the first statement of each match arm in `open_settings` today; theme is `config.theme = Some(self.theme.clone())`), and `impl Default` whose values are exactly the right-hand sides in today's `fn reset` (theme is `theme::NAMES[0].into()`). If a type lacks `PartialEq`, derive it on that type in `src/config.rs` or `src/shaders.rs`.

   `Settings` is NOT a field of `Config` and gets no serde derive: `config.json` must not change.

2. `SettingsView` holds one `settings: Entity<Settings>` in place of the thirteen entities, and one observer in place of `_observe: [Subscription; 13]`. The observer:
   - reads the new value;
   - calls `theme::apply(&new.theme, cx)` only when the theme differs from the previous value (keep the previous value in a `shown: Settings` field on `SettingsView`);
   - emits `SettingsChanged(new.clone())`.

3. `pub enum SettingsChanged` becomes `pub struct SettingsChanged(pub Settings);`.

4. Every `SettingField` closure that reads or writes one of the old entities reads or writes the matching field of `self.settings` instead. Keep each closure's logic; only the target changes. Keep the `cx.notify()` after each write: it is what fires the observer.

5. `fn reset` becomes one update: `*settings = Settings::default(); cx.notify();`. Delete the local `set` helper.

6. In `AppView::open_settings` (`src/app.rs`), the subscription body becomes, in this order:
   - `let old = Settings::from_config(&this.config);`
   - `new.write_to(&mut this.config);`
   - `cx.set_global(new.seek); cx.set_global(new.hide_spoilers); cx.set_global(new.auto_skip);`
   - `main.sync_backdrops(new.backdrops.0, cx)` only when `new.backdrops != old.backdrops`
   - `main.home.update(.. set_layout ..)` only when `new.home_layout != old.home_layout`
   - the `set_grouping` loop over `[&main.movies, &main.series]` only when `new.library_grouping != old.library_grouping`
   - `save_config(&this.config, "settings");`

   The three comparisons matter: `set_layout` and `set_grouping` may reload data, and today they run only when their own setting changes.

## Steps

1. Add `Settings` with `from_config`, `write_to` and `Default`, and a unit test: `Settings::from_config(&config)` followed by `write_to` on a second default config leaves the thirteen fields equal to the first config's.
2. Convert `SettingsView` (design points 2 to 5), one page function at a time, compiling between pages.
3. Convert `open_settings` (design point 6).
4. Confirm nothing else references the old variants: `rg -n "SettingsChanged::" src` prints nothing.

If any setting behaves in a way this design does not cover (for example a write that must not go through the observer), stop and describe it under Status.

## Done when

- [x] `rg -n "SettingsChanged::" src` prints nothing, and `_observe: [Subscription; 13]` is gone
- [ ] Changing each of the thirteen settings still saves it to `config.json` and takes effect as before (theme switches at once; layout and grouping change Home, Movies and Series)
- [ ] Reset all settings still restores every default
- [x] A `config.json` written before this change loads unchanged, and saving it changes no key name
- [x] `src/app.rs` is under 1000 lines

**Check:** the gate in `PLAN.md`. Runtime: change five settings, reset, and diff `config.json` before and after.
