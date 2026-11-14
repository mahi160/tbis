# 09 — Video quality setting (Fast / Balanced / High)

**What to build:** tbis sets no mpv rendering profile, so every machine gets mpv's defaults. Add one Settings choice that applies an mpv built-in profile at Player start: **Fast** (`fast`, for weak GPUs and battery), **Balanced** (mpv defaults, the default here) and **High** (`high-quality`, better scaling and debanding).

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). Differs from step 3: the profile is passed as the `profile` option in the Player's `defaults` (before `mpv_initialize`), not `apply-profile` after it. That keeps the documented order (Settings first, the user's `mpv.conf` overrides them, as Settings > mpv says), needs no `Mpv::new` parameter, and never touches `vo` (neither profile sets it), so the `EMBEDDING` re-apply condition is unchanged. Test `profile_option_before_init_applies_the_profile` proves libmpv applies it.

**Read first:** `src/config.rs` (`AudioStyle`, `Config`, `fn defaults`), `src/settings.rs` (`SettingsChanged`, `SettingsView::new`, `fn video_group`), `src/app.rs` (`fn open_settings`), `src/player/mod.rs` (`PlayerView::new`, the `defaults` vector), `src/mpv/mod.rs` (`Mpv::new`, `fn init`, `fn command`), `src/shaders.rs` (`ShaderProfile` is the pattern for a small enum with labels).

## Steps

1. Confirm the profile names: `mpv --profile=help` must list `fast` and `high-quality`.
2. In `src/config.rs` add:

   ```rust
   /// mpv rendering profile the Player starts with (Settings).
   #[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
   pub enum VideoQuality {
       Fast,
       #[default]
       Balanced,
       High,
   }

   impl VideoQuality {
       pub const ALL: [Self; 3] = [Self::Fast, Self::Balanced, Self::High];

       pub fn label(self) -> &'static str {
           match self {
               Self::Fast => "Fast",
               Self::Balanced => "Balanced",
               Self::High => "High",
           }
       }

       /// mpv built-in profile to apply; `None` keeps mpv's defaults.
       pub fn mpv_profile(self) -> Option<&'static str> {
           match self {
               Self::Fast => Some("fast"),
               Self::Balanced => None,
               Self::High => Some("high-quality"),
           }
       }
   }
   ```

   Add `#[serde(default)] pub video_quality: VideoQuality,` to `Config` and `video_quality: VideoQuality::default(),` to `fn defaults`.
3. In `src/mpv/mod.rs`, give `Mpv::new` a new parameter `profile: Option<&str>` and pass it to `init`. In `init`, right after `mpv_initialize` succeeds and before the block that re-applies `EMBEDDING`, add:

   ```rust
   if let Some(profile) = profile {
       self.command(&["apply-profile", profile])?;
   }
   ```

   Applying it before the `EMBEDDING` block keeps `vo=libmpv` in force. When no user `mpv.conf` exists that block is skipped, so also re-apply `EMBEDDING` whenever a profile was applied: change its condition from `user_config.is_some()` to `user_config.is_some() || profile.is_some()`.
4. Update every caller of `Mpv::new` (`rg -n "Mpv::new" src`): the Player passes `config.video_quality.mpv_profile()`.
5. Settings: follow the `shaders` wiring exactly. Add `SettingsChanged::VideoQuality(VideoQuality)`, an `Entity<VideoQuality>` field, its `cx.observe` (the `_observe` array grows by one, so update its length in the struct), and a dropdown item "Quality" in `fn video_group` above "Upscaling" with description "Fast saves power; High looks best on a capable GPU. Applies to the next video."
6. In `src/app.rs`, `fn open_settings`, add the match arm `SettingsChanged::VideoQuality(quality) => this.config.video_quality = *quality,`.
7. Add a test in `src/config.rs` tests: a config JSON without `video_quality` loads as `Balanced`. Copy the shape of `playback_preferences_preserve_old_track_memory`.

## Done when

- [x] Settings > Playback > Video shows Quality with three choices, saved to `config.json`
- [x] The Player applies `fast` or `high-quality` (as the `profile` option, see Status); Balanced applies nothing
- [x] `vo` is still `libmpv` after a profile is applied (neither profile sets `vo`)
- [x] Old configs load as Balanced (test)

**Check:** the gate in `PLAN.md`. Runtime: set High, play a video, press `i`; playback info still shows video and no error.
