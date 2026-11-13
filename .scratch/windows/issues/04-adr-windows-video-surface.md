# 04 — ADR: Windows video surface

**What to build:** `docs/adr/0007-windows-video-surface.md`. It supersedes ADR-0001's macOS-only paragraph and records: our own D3D11 + DirectComposition device with `CreateTargetForHwnd(hwnd, false)` beneath gpui's topmost target; mpv rendering GL through WGL + `WGL_NV_DX_interop2` into a swap chain on that visual; the transparent gpui window drawing controls on top; `hwdec=d3d11va,d3d11va-copy,no`; the fallback (gpui fork with an external-texture element); and the libmpv pin (source, version, how `mpv.lib` is made, `libmpv-2.dll` shipped next to the exe, GPL per ADR-0004). Written as "proposed" first, then marked "accepted" with evidence from ticket 05.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] ADR-0001 links to the new ADR for Windows
- [ ] The libmpv pin and the mpv.lib recipe are recorded
- [ ] Status is updated with ticket 05's result
