# Cross-platform plan: Windows and Linux (Wayland + X11)

Branch: `cross-platform`. Today tbis is macOS-only: `src/mpv/mod.rs` has a `compile_error!` on other targets and `src/mpv/mac.rs` is the only video surface (ADR-0001).

## Facts checked in the repo and in gpui-pre 0.3.8

- gpui's default features include `wayland` and `x11`. The Linux backend is the `gpui-pre-linux` crate and the Windows backend is `gpui-pre-windows`.
- Window handles (raw-window-handle):
  - Wayland returns `WaylandWindowHandle` and `WaylandDisplayHandle`, so a `wl_subsurface` under gpui's surface is possible.
  - X11 returns xcb handles.
  - Windows returns `Win32WindowHandle` (HWND).
- gpui's `surface()` element is macOS/iOS only (`CVPixelBuffer`). There is no external-texture element on Linux or Windows.
- The Windows renderer already uses DirectComposition (`directx_renderer.rs`: `IDCompositionVisual`). That makes a video visual beneath gpui's visual plausible. Still unverified.
- `WindowBackgroundAppearance::Transparent` is handled on Windows and Wayland. The X11 path is not checked yet.
- gpui-kit 0.7.1 is not a blocker. It enables `gpui_platform` with `x11`, `wayland` and `runtime_shaders` on every desktop target. It was built for cross-platform use:
  - Its `TitleBar` draws Windows and Linux window-control buttons itself (`gpui-component/src/title_bar.rs`).
  - Its `window_border` has a Linux-specific client-side-decoration path.
  - `Kbd` renders `ctrl`/`alt` labels on non-macOS.
  - Its native menus have macOS and Windows implementations only. Check whether tbis uses them (Linux status unverified).
- `src/now_playing.rs` and `src/player/mod.rs` already have `cfg(not(macos))` stubs. The `objc2-*` dependencies are already macOS-gated.

## Phase 0: make it build (no video)

Goal: `cargo check` passes on Linux and Windows targets, and the UI runs with a "video unsupported" stub.

- [x] `src/mpv/mod.rs`: remove the `compile_error!`. Select the `platform` module by `cfg(target_os)`.
- [x] Add a stub `platform` module for other targets. `Surface::attach` returns `Err`, and the Player shows the error.
- [x] `src/support_dir.rs`: use `dirs`. That means XDG on Linux and `%APPDATA%` on Windows. Drop the `HOME` panic.
- [x] `src/player/controls.rs`: screenshot dir via `dirs::picture_dir()`.
- [x] `src/main.rs`: bindings use `cmd-` on macOS and `ctrl-` elsewhere. Keep `cfg(unix)` for the rlimit.
- [x] `src/update.rs`: no change needed. `bundle_path()` is `None` outside a `.app`, so the updater is already inert (its notice still says "tbis.app"; reword later).
- [x] `src/pip.rs`: gate or stub the `UnixStream` use, so it compiles on Windows.
- [ ] `build.rs` and `README.md`: document libmpv dev files per OS.

Verified on macOS only (`cargo check`). This machine has no cross-target std (Homebrew rust, no rustup), so the Linux/Windows compile check is still to do.

Done when: `cargo check --target x86_64-unknown-linux-gnu` and `--target x86_64-pc-windows-msvc` pass, and the app launches on a Linux box with the UI usable.

## Prior art: ../photon (Tauri app that ran on Linux and Windows)

Read: `docs/adr/0005`, `0006`, `0009`, `0010`, `0011`, `src-tauri/src/mpv/{linux,windows}`, `src-tauri/src/idle.rs`, `.github/workflows/release.yml`.

- **Photon tried our Spike A and dropped it.** Its `wayland.rs` (see git history, commit `a0b3fee`) is almost the same design as ours: `wl_subsurface` + EGL on gpui's/GTK's display. It was written blind, never run, and ADR-0010 says it failed in the field: the video escaped into its own top-level window. ADR-0010 blames a second `wl_client`, but `from_foreign_display` wraps the same `wl_display`, so that diagnosis looks wrong. Unverified. Treat Spike A as higher risk than I first rated it.
- **Photon's working Linux fix was a toolkit-composited surface** (`GtkGLArea` under the webview). gpui has no equivalent, so our analogue is the external-texture fallback (fork of gpui).
- **Photon's working Windows path** is a WGL child HWND, a sibling of WebView2's HWND, with `HWND_BOTTOM` ordering. It works there because WebView2 is its own HWND. gpui draws through DirectComposition into the top-level HWND, so the z-order assumption does not carry over. Spike B must test it.
- **Reusable now:**
  - Windows WGL code (`windows/mod.rs`): `CS_OWNDC`, `opengl32.dll` fallback for core GL functions, `GetDpiForWindow / 96` for scale.
  - Pass `MPV_RENDER_PARAM_WL_DISPLAY` / `X11_DISPLAY` to `mpv_render_context_create`, and set `hwdec=vaapi,nvdec,vaapi-copy,nvdec-copy,no` after init. Not `auto-safe`: it probes vulkan and cuda on every load.
  - Linux GL symbols from `eglGetProcAddress` / `glXGetProcAddressARB`, never libepoxy (its pointers are never NULL and it aborts on missing functions).
  - `skip_frame` when hidden, so mpv does not stall (we already do this).
  - Linux sleep inhibit: `org.freedesktop.ScreenSaver.Inhibit` over D-Bus (`idle.rs`). For `platform/linux/power.rs`. gpui-linux already depends on `ashpd`.
  - PiP on Wayland: `--ontop` and `--on-all-workspaces` are no-ops there (ADR-0006).
  - CI: Windows has no pkg-config libmpv. Download shinchiro's `mpv-dev-x86_64-*.7z`, `gendef` the dll, `lib /def` to make `mpv.lib`, set `MPV_LIB_DIR`, copy `libmpv-2.dll` next to the exe. Linux: `apt install libmpv-dev`, `.deb` with `depends: libmpv2` (no mpv code shipped), no AppImage (it bundles libva).
  - Licensing (ADR-0011): macOS and Windows binaries are GPL because they bundle GPL libmpv; Linux ships none.

## Module layout (done)

Per-OS code lives in `src/platform/{macos,linux,windows}/`. Each exposes `video`, `now_playing`, `power`, `cursor`, `window`; the app only calls `crate::platform::*`. Features an OS hasn't built use `platform/noop.rs`. Linux `video/` dispatches to `wayland.rs` or `x11.rs` by window handle.

## Phase 1: video surface spikes (throwaway, timeboxed)

Each spike answers one question. Record results in a new ADR that supersedes the "macOS only" part of ADR-0001.

- [ ] **Spike A: Wayland.** Code written in `src/platform/linux/video/wayland.rs` (type-checked against the real wayland/EGL crates, never run). Needs a run on a Wayland compositor. A `wl_subsurface` below gpui's surface, fed by mpv. Questions:
  - Does a transparent gpui window show the subsurface through it?
  - Does mpv's render API (GL via EGL, dmabuf) present into the subsurface?
- [ ] **Spike B: Windows.** Try an mpv visual in the same DirectComposition tree beneath gpui's visual. mpv would render GL through WGL, with DX interop into a swap chain. Questions:
  - Is the gpui window transparent via DComp?
  - How is the visual order controlled? gpui's renderer owns the DComp device, so we may need a second HWND or a gpui patch.
- [ ] **Spike C: X11.** Questions:
  - Does gpui's ARGB visual plus a compositor let a window beneath show through?
  - If not, fall back to a separate mpv window with a transparent overlay window.
- [ ] **Fallback decision.** If spikes A to C fail, fork gpui-pre and add an external-texture element (dmabuf on Linux, D3D11 shared texture on Windows). That is the same mechanism on every platform but the largest cost.
- [ ] Check `libmpv-sys` / mpv render API support for the chosen GL/EGL/D3D path on each OS.

Done when: one ADR states the chosen surface strategy per platform, with evidence from a running spike.

## Phase 2: real surfaces

- [ ] `src/mpv/linux_wayland.rs`, `src/mpv/linux_x11.rs`, `src/mpv/windows.rs`, each exposing `Surface` and `Renderer` with the same API as `mac.rs`. Runtime pick between Wayland and X11 from the window handle type.
- [ ] Reuse `create_gl_render_context` and `render_gl` from `mpv/mod.rs`. Only surface and present code is per-platform.
- [ ] Transparent-window toggle when the Player is shown (already done for macOS in `main.rs`; confirm it behaves on each backend).
- [ ] Manual checks: 1080p SDR, 4K, resize, fullscreen, window move, HiDPI scale, subtitles drawn.

## Phase 3: platform integrations

- [ ] Media keys and now-playing: MPRIS via zbus on Linux, SMTC on Windows (`src/now_playing.rs`).
- [ ] PiP (`src/pip.rs`): named pipe IPC on Windows. On Wayland `--ontop` and `--on-all-workspaces` are not honoured, so decide on a degraded behaviour or drop PiP there. mpv binary lookup via `PATH` (Linux) or next to the exe (Windows).
- [ ] Prevent sleep during playback: Linux done (`platform/linux/power.rs`, ScreenSaver.Inhibit over zbus, never run). Windows still `SetThreadExecutionState`.
- [x] Per-platform `hwdec` (`video::HWDEC`) and the Wayland display handed to mpv for hardware-decode interop (never run).
- [ ] Hide cursor during playback (`player/mod.rs`): check gpui's cursor API per backend.
- [ ] Title bar: `TITLE_BAR_INSET` and the drag region on Windows and on Wayland client-side decorations.
- [ ] Fonts for subtitles (`src/fonts.rs`): confirm libass finds the written font dir.

## Phase 4: packaging, CI, updates

- [ ] CI matrix in `.github/workflows/`: Linux (Ubuntu with mpv dev, wayland, xkbcommon, xcb, vulkan libs) and Windows (libmpv dev package). `cargo test` and `cargo check` on both.
- [ ] Linux artifact: AppImage or Flatpak, `.desktop` file, icon. Link system libmpv. Update ADR-0004 for the GPL implications per platform.
- [ ] Windows artifact: zip or installer with `libmpv-2.dll` and ffmpeg DLLs, `.ico` (replace `render-icon.swift` flow for these).
- [ ] Updater per platform, or leave to the package manager.
- [ ] README: install instructions per OS.

## Order and risk

1. Phase 0 is routine. Do it first so every later spike runs on a building tree.
2. Phase 1 is the real risk. It decides whether we need the gpui fork.
3. Phases 3 and 4 can run in parallel once one platform has video.

## Open questions

- Is Linux packaging Flatpak, AppImage, or both?
- Is the Windows target x64 only, or also arm64?
- Is macOS Intel in scope while we are touching CI?
- Do we accept PiP being unavailable on Wayland?
