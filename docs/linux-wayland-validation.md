# L2: Validate native Wayland playback

**Status: awaiting a real Wayland desktop test.** An L1 build, an emulated compositor, or an XWayland window does not pass this ticket.

## Prepare and launch

1. On an x86-64 Linux desktop, log into a native Wayland session and open a terminal there. Build the revision under test using the [Linux build instructions](../README.md#linux-build-and-test-ubuntu-2404-x86-64).
2. Add a redistributable clip longer than one minute with motion, audio, and clear sync cues to a Jellyfin test library. Record its identifier and SHA-256 so the exact same file can be used for X11 and XWayland later. Sign into that server in tbis. Record whether Jellyfin direct played, remuxed, or transcoded it.
3. From the repo root, launch with X11 unavailable and capture stderr. Keep the log local; review it for credentials, private URLs, and paths before sharing excerpts. Use a fresh data directory to keep local settings and `mpv.conf` out of the baseline.

```sh
umask 077
run_dir=$(mktemp -d)
git rev-parse HEAD
rustc -Vv
uname -sm
env -u DISPLAY XDG_DATA_HOME="$run_dir" WAYLAND_DEBUG=client \
    ./target/release/tbis 2>"$run_dir/wayland.log"
```

While the app is running, open the clip briefly and then quit. Check the log:

```sh
grep -E 'wl_subcompositor.*get_subsurface|wl_subsurface.*place_below|video surface attachment failed|hwdec-current=' "$run_dir/wayland.log"
```

The [video dispatcher](../src/platform/linux/video/mod.rs) takes the Wayland branch only for `RawWindowHandle::Wayland`. In this checkout, `get_subsurface` followed by `place_below` when playback opens is evidence that the [Wayland video branch](../src/platform/linux/video/wayland.rs) ran. It is not evidence of a presented frame. If the trace is inconclusive, report the backend as unconfirmed; do not infer it from `XDG_SESSION_TYPE` or `WAYLAND_DISPLAY`. If attachment fails, record the first error and the action that triggered it. Do not retry in XWayland for an L2 pass.

## Playback checklist

Reopen without the trace to avoid measuring its overhead. Keep `DISPLAY` unset and reuse the same data directory and clip:

```sh
env -u DISPLAY XDG_DATA_HOME="$run_dir" ./target/release/tbis 2>"$run_dir/playback.log"
```

- Play with audio for at least one minute. Look for moving video, audio/video sync, and no sustained stutter.
- Pause, seek, and resume. Click an on-video control and confirm it responds; the video subsurface should not intercept clicks.
- Resize the window and enter and exit fullscreen twice. Check that the video stays aligned with the player area and does not cover decoration insets incorrectly. Record the monitor scale; if fractional scaling is available, repeat resize at that scale.
- Close playback and reopen the clip. Check for a stranded surface, a blank reopen, or a crash.
- Repeat with software decoding: put `hwdec=no` in the Settings `mpv.conf` field and reopen playback so mpv initializes with the new setting. Check the player's **Hardware decoding** stats or the `mpv: hwdec-current=` line; record that it was off. Clear the test setting afterward. Smooth software playback counts as a pass.

The resize and teardown checks target specific code paths: the renderer uses a viewport when available and otherwise rounds to an integer buffer scale; the render thread destroys the EGL surface before the main thread destroys the Wayland surface. The code documents that it currently ignores client-side decoration insets. These are inspection findings, not observed failures.

## Record the result

```text
Revision / local changes:
Machine, GPU, driver, distro, compositor/version, monitor scale:
Clip ID, SHA-256, resolution, fps, video/audio codecs:
Jellyfin playback mode:
Wayland backend evidence:
Default decoder / software decoder evidence:
One-minute playback, sync and stutter (default / software):
Pause, seek, resume and overlay input:
Resize and two fullscreen cycles:
Close and reopen:
First failure, steps to reproduce, and redacted log excerpt:
Native Wayland result: pass / fail / unverified
```

Update the native Wayland row in the [Linux support plan](linux-support-plan.md) only with observed results. Leave X11, XWayland, and Flatpak rows untouched.
