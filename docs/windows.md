# Windows validation status

The Windows video architecture is [proposed in ADR-0007](adr/0007-windows-video-surface.md), not accepted. Ticket 01 Windows smoke testing remains unrun. Actual native Windows UI/app-window, login, network playback, GPU interoperability, SMTC/power/cursor behavior on a human PC and screenshots remain **unverified**, as does the GPU matrix; the CI self-test is GPU-free.

Build steps, toolchain and CI status are in [Windows builds and releases](windows-build.md). Application release cross-building from macOS remains blocked by gpui's Windows-host-only HLSL generation (`shaders_bytes.rs` is not produced there).

## Portability audit

- Shader `glsl-shaders` lists use `;` on Windows and `:` elsewhere. The output-seam test uses literal complete path-list expectations, including Windows drive-letter paths. The test passed in the macOS release suite; shader application on Windows is unverified.
- `support_dir::app_support_dir()` uses `dirs::data_dir().join("tbis")`; shader, font and mpv support paths are joined as filesystem paths, not split on drive-letter colons. No support-directory change was needed.
- Screenshots use `dirs::picture_dir().join("tbis")` when Pictures is available and pass the resulting filename as one `screenshot-to-file` command argument. This is not an mpv path list. The existing Pictures location is unchanged; actual Windows file creation is unverified.
- Windows PiP has no button, binding, help row or action handler. The PiP module is excluded on Windows, preventing that path from invoking unsupported surface movement. macOS/Linux retain their existing PiP implementation; regression acceptance remains unverified.
- Only macOS receives the traffic-light position and extra brand lead. gpui-kit's installed TitleBar supplies Windows caption controls through `WindowControlArea` and a drag region; these are retained rather than replaced. Its Kbd formats Control as `Ctrl` outside macOS, and the app binds platform shortcuts with `secondary`. The repository search found no hardcoded Command glyph in UI text, only a comment. Actual rendered hints remain unverified.

## Manual acceptance matrix

Record Windows version, GPU model/driver, display scale, media codec/resolution, active decoder, observed frame drops, logs and screenshots with each result. A successful build or `--self-test` is not GPU evidence.

| Acceptance | Intel | AMD | NVIDIA |
| --- | --- | --- | --- |
| Ticket 01 launch and Windows smoke test | Unrun | Unrun | Unrun |
| WGL OpenGL 3.3 / `WGL_NV_DX_interop2` initialization | Unverified | Unverified | Unverified |
| Video beneath transparent controls, no black/occluded video | Unverified | Unverified | Unverified |
| `d3d11va`, `d3d11va-copy`, software fallback and dropped frames | Unverified | Unverified | Unverified |
| Resize, minimize/restore, fullscreen, repeated open/close | Unverified | Unverified | Unverified |
| Device/render failure shown; close/reopen recovers | Unverified | Unverified | Unverified |
| Anime4K fast/quality apply; Off clears shaders | Unverified | Unverified | Unverified |
| Screenshot saved under Windows Pictures/tbis | Unverified | Unverified | Unverified |
| PiP absent from controls/help; P does not invoke PiP | Unverified | Unverified | Unverified |
| Cursor restored after leaving playback | Unverified | Unverified | Unverified |
| Brand, centered content, caption buttons, border at 100%/150% | Unverified | Unverified | Unverified |
| Drag, double-click maximize, caption actions and Aero snap | Unverified | Unverified | Unverified |
| Ctrl hints; no Command glyph in rendered UI | Unverified | Unverified | Unverified |

Screenshot acceptance is explicitly **unverified**. Before using screenshots as evidence, confirm the binary was freshly recompiled (`Compiling tbis`, not merely `Finished`) as required by the repository UI instructions. macOS/Linux PiP and existing title-bar appearance also need manual regression acceptance.
