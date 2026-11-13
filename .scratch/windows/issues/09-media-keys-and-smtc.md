# 09 — Media keys and System Media Transport Controls

**What to build:** `platform::windows::now_playing::NowPlaying` via SMTC (`SystemMediaTransportControls` through `ISystemMediaTransportControlsInterop::GetForWindow` on the main HWND): title, series/episode, artwork, play/pause state, position, and next/previous enabled from `set_episode_nav`. Button presses go to the same `RemoteCommand` channel as macOS.

**Blocked by:** 06 (needs playback to test).

**Status:** needs-human (code landed; manual checks pending)

- [ ] Keyboard media keys play, pause, and skip episodes
- [ ] The Windows media flyout shows title and artwork, and its buttons work
- [ ] Closing the Player clears the flyout
