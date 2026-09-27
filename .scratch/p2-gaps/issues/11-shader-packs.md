# 11 — Shader packs (upscaling)

**What to build:** A Video section in Settings offers upscaling shader profiles (e.g. Anime4K low/high quality, FSRCNNX) bundled or downloaded with the app, plus "Off". The chosen profile applies in the Player; a Player shortcut toggles shaders on/off instantly, with a toast naming the active profile.

**Blocked by:** p1-gaps 06 — Settings screen with default language; 01 — Toasts for transient messages.

**Status:** ready-for-agent

- [ ] Profile picker in Settings (including Off), saved to config
- [ ] Profile applies on playback start; shortcut toggles it live
- [ ] Shader licenses noted where they are bundled
- [ ] Missing/failed shader load falls back to no shaders with a toast, playback continues
