---
status: accepted
---

# PiP moves the Player's video surface into a floating window

This supersedes ADR-0003. Pressing PiP no longer launches a standalone `mpv`. The Player's own in-process mpv keeps playing, and only the native video surface from ADR-0001 moves into a second, small gpui window. On macOS that surface is the `NSView` holding the `CAMetalLayer`. On Wayland it is the `wl_subsurface`, which is re-created under the new parent `wl_surface`. Leaving PiP moves the surface back. mpv's render context, decoder, cache and position are untouched, so entering and leaving PiP is instant, and there is nothing to keep in sync between two players.

The PiP window is a gpui `WindowKind::PopUp` window with a transparent title bar, which keeps it resizable. Its title bar buttons are hidden, so it shows only the video, like mpv's own window. On macOS that is a non-activating panel that floats above other windows and joins every Space, including full-screen ones. Its controls (play/pause, back to the Player) are drawn by tbis, like the Player's. Keyboard shortcuts keep going to the Player window, so P still toggles PiP.

## Considered Options

- **Spawned standalone mpv (ADR-0003)**: rejected because it reloads the stream from scratch. The new process has to connect, probe the file, seek and refill its cache, which takes seconds on a 4K remux. It also needs JSON IPC to sync the position and hands the auth token to a second process.
- **Native OS PiP (AVKit on macOS)**: rejected because it is macOS-only and needs mpv's frames bridged into an AVKit-compatible layer. Linux and Windows would need a separate feature.
- **Pre-launching an idle standalone mpv**: rejected because it only hides process start-up. The stream still reloads.

## Consequences

- One mpv process per Player. The bundle no longer ships the `mpv` binary (ADR-0004), only libmpv and its dependencies.
- Always-on-top is not guaranteed on Wayland, where the compositor decides window stacking. The Wayland surface move is also unverified on a real compositor.
- Each platform's `video::Surface` must implement `move_to`. A platform without in-window video (X11 and Windows today) has no PiP either.
- PiP closes when the Player closes, because the video belongs to the Player.
