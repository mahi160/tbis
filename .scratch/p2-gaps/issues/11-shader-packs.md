# 11 — Shader packs (upscaling)

**What to build:** A Video section in Settings offers upscaling shader profiles (e.g. Anime4K low/high quality, FSRCNNX) bundled or downloaded with the app, plus "Off". The chosen profile applies in the Player; a Player shortcut toggles shaders on/off instantly, with a toast naming the active profile.

**Blocked by:** p1-gaps 06 — Settings screen with default language; 01 — Toasts for transient messages.

**Status:** done (runtime unverified)

Bundled Anime4K v4.0.1 only (MIT), as Off / Anime4K (fast) / Anime4K (quality) using its official Mode A chains. FSRCNNX was left out: it is LGPL-3.0, and mpv's built-in scalers already handle live action well.

- [x] Profile picker in Settings > Playback > Video (including Off), saved to config
- [x] Profile applies on playback start; `U` cycles Off / fast / quality live, with a toast naming the profile. PiP starts with the active profile.
- [x] Shader licenses noted where they are bundled (assets/shaders/anime4k/LICENSE)
- [x] Missing shader files (write failure) or a rejected `glsl-shaders` value fall back to no shaders with a toast, and playback continues. A shader that fails to compile on the GPU is only logged by mpv.
