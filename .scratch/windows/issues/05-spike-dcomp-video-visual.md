# 05 — Spike: mpv frame in a DirectComposition visual beneath gpui

**What to build:** Throwaway, timeboxed. In `src/platform/windows/video.rs`, from the window's HWND: create a D3D11 device and a DComp device, `CreateTargetForHwnd(hwnd, false)`, a flip-model swap chain for composition on a visual, a WGL context with `WGL_NV_DX_interop2` sharing the swap chain's back buffer, and have mpv render one stream into it. Answers: does gpui's transparent window (`WindowBackgroundAppearance::Transparent`) let the video show through; is the bottom target really drawn under gpui's topmost one; does interop work on Intel, AMD and NVIDIA. If any answer is no, the result is a write-up recommending the fallback (gpui fork with an external-texture element) instead.

**Blocked by:** 01 (the UI must run on Windows at all).

**Status:** needs-human (code landed; run each build on real GPUs)

- [ ] A playing video is visible under the Player controls, from a CI build
- [ ] Findings (what worked, which GPUs, errors) are written into ticket 04's ADR
- [ ] Go / no-go decision recorded
