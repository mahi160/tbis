---
status: accepted
---

# PiP hands playback to a spawned standalone mpv

This is the only exception to ADR-0001's rule that nothing is spawned. Pressing PiP pauses the in-process Player and launches the `mpv` binary bundled in the `.app` (ADR-0004; a development build falls back to Homebrew's) using `--ontop --on-all-workspaces --no-border`, starting at the current position. When that window closes, the Player resumes at the position the spawned mpv reported over its JSON IPC (`--input-ipc-server`). We chose this over real macOS PiP (AVKit), which would require bridging mpv's frames into an AVKit-compatible layer, and over a custom mini-window, because mpv's own window already provides borderless, always-on-top, draggable, and resizable behavior. The approach follows Photon ADR-0006.
