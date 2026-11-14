# 07 — Spike: mpv draws into a child window on Windows

**What to build:** A throwaway proof that Windows video can skip the render API. Today Windows renders through WGL, copies through DX interop into our own swap chain, and needs about 75 `unsafe` uses across `wgl.rs` and `video.rs`. The simpler design: create a plain child window inside the gpui window and give its handle to mpv with the `wid` option, so mpv presents with its own Direct3D 11 output (`vo=gpu-next`, `gpu-api=d3d11`). That brings native D3D11 decoding and HDR, and deletes the interop code.

It can work because gpui's composition target is created with `topmost = true` (`gpui-pre-windows-0.3.8/src/directx_renderer.rs`, `CreateTargetForHwnd(hwnd, true)`): a topmost target is drawn above the window's child windows, so the controls should stay on top of the video.

**Model:** opus + human (needs a Windows PC with a GPU)

**Blocked by:** 06 (run it only when the log shows the interop path is the problem, or when the owner wants HDR on Windows).

**Status:** blocked

**Read first:** `docs/adr/0007-windows-video-surface.md`, `docs/adr/0001-mpv-native-layer-under-gpui.md`, `src/platform/windows/video.rs`, `src/mpv/mod.rs` (`EMBEDDING`, `Mpv::new`, `Handle::init`), `src/mpv/render.rs`.

## Questions the spike must answer

1. Does a child `HWND` with mpv's own D3D11 swap chain show under gpui's controls, with the controls still visible and clickable?
2. Do mouse moves and clicks over the video reach gpui? (Expected fix: create the child disabled, `WS_DISABLED`, so input falls through to the parent.)
3. Does resize, fullscreen, minimize and restore keep the child filling the window?
4. Is `hwdec=d3d11va` active (`hwdec-current` in the log), and does an HDR file switch the display to HDR with `target-colorspace-hint=yes`?
5. Does it work on Intel, AMD and NVIDIA, and on a laptop with two GPUs?

## Steps

1. On a branch, behind an environment variable `TBIS_WINDOWS_WID=1`, create the child window in `Surface` creation and skip the render thread on that path.
2. Pass `wid` (the child handle as a decimal string), `vo=gpu-next`, `gpu-api=d3d11` instead of `vo=libmpv`. `EMBEDDING` forces `vo=libmpv` today; the spike needs a per-platform value.
3. Resize the child from the same place `VideoSize::set` is called.
4. Record the answers, with screenshots and the log, in a new ADR that supersedes ADR-0007 when the answers are yes.

## Done when

- [ ] All five questions have a written answer backed by a log or screenshot
- [ ] A new ADR records go or no-go
- [ ] On go: a follow-up ticket lists what to delete (`wgl.rs`, the interop half of `video.rs`) and what the libmpv build must enable (`d3d11`, `gpu-next`; see `scripts/windows-deps.json`, where `libplacebo:d3d11` is disabled today)
