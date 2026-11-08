# 06 — Windows video surface (Surface + Renderer)

**What to build:** The real `platform::windows::video` with the same API as macOS: `Surface::attach`, `Renderer::init` / `draw`, built on ticket 05's go decision (or the fallback). Handles window resize, maximize and fullscreen, moving between monitors with different DPI (`GetDpiForWindow`), device loss (recreate on `DXGI_ERROR_DEVICE_REMOVED`), and `skip_frame` when the window is minimized. `HWDEC = "d3d11va,d3d11va-copy,no"`. `Surface::move_to` returns `Err` (PiP is out of scope). Errors and frame stats go to `tbis.log` (ticket 02).

**Blocked by:** 02, 05.

**Status:** needs-human (code landed; manual checks pending)

- [ ] 1080p SDR and 4K HEVC play smoothly with hardware decode (decoder named in the log)
- [ ] Resize, maximize, fullscreen and monitor moves keep the video aligned under the controls
- [ ] Subtitles draw
- [ ] Closing the Player tears everything down; opening it again works
- [ ] Manual checks run on at least one Intel/AMD and one NVIDIA machine
