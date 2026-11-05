---
status: proposed
---

# Render Windows video beneath gpui with D3D11 and DirectComposition

This supersedes the Windows paragraph of [ADR-0001](0001-mpv-native-layer-under-gpui.md). Status is **PROPOSED**, not accepted: the implementation exists, but ticket 05 GPU evidence is unavailable and ticket 01 Windows smoke testing remains unrun. Source inspection is not proof that the composition or driver interop works on hardware.

## Proposed architecture and current implementation

`src/platform/windows/video.rs` creates its own hardware D3D11 device, DXGI composition swap chain, DirectComposition device, visual and HWND target. `CreateTargetForHwnd(hwnd, false)` requests a bottom target beneath gpui's topmost target. The Player makes the gpui window transparent and draws controls above the video. The intended result is one visible window, without a child video HWND covering the controls.

`src/platform/windows/wgl.rs` creates a hidden helper HWND/DC, bootstraps WGL, then creates an OpenGL 3.3 core context. It requires the advertised `WGL_NV_DX_interop2` extension and loads the GL framebuffer and WGL/DX interop entry points. The extension name is not evidence of support on any particular GPU vendor. Missing extensions or entry points fail initialization; adapter mismatch can also make `wglDXOpenDeviceNV` fail.

The implementation does not render directly into a swap-chain buffer. It registers an intermediate BGRA8 D3D11 texture with WGL, locks it for GL, and attaches it to an FBO for libmpv's OpenGL render API. After rendering and unlocking, D3D11 `CopyResource` copies that texture into the current swap-chain backbuffer, followed by `Present(1, ...)`. The swap chain uses two buffers, flip-sequential presentation and ignored alpha. This path does not establish HDR output support or zero-copy presentation.

The configured decoder preference is `hwdec=d3d11va,d3d11va-copy,no`: native D3D11 decoding, copy-back decoding, then software decoding. The active decoder and dropped-frame counters must be recorded during hardware acceptance; a configured preference does not prove hardware decoding. Software decoding still needs the same WGL/D3D presentation interop.

Windows PiP is intentionally unavailable. Its button, key binding, shortcut-help row and action handler are gated out, along with the PiP window module. Windows `Surface::move_to` remains unsupported and is not a fallback rendering path.

## Ownership, threading and teardown

`Surface` borrows the gpui HWND; it does not own or destroy that window. `Renderer` moves to the render thread before initialization. Its COM apartment, WGL context, D3D11 device/context, interop handle, composition objects and textures are created, used and dropped on that thread. The HWND must remain alive until mpv teardown has joined rendering.

`Runtime` owns the texture before the composition objects, interop device, GL context and COM apartment in drop order. Texture teardown unlocks if necessary, deletes the FBO, unregisters the interop object, then deletes the GL texture name. `Rc<Interop>` keeps the D3D device and interop handle alive through texture teardown. Composition teardown detaches its root and commits; interop teardown closes the WGL/DX device before the WGL context is destroyed. WGL teardown releases the context, DC and helper window; the COM apartment is uninitialized last. Failed partial initialization unwinds through these owned resources. Interop unregister/close failures are logged; other teardown operations are best-effort.

`Mpv::drop` signals stop, wakes and joins the worker threads, and destroys the mpv handle only after the render thread has freed the mpv render context and dropped its renderer. Dropping the Windows `Surface` restores the cursor.

## Failure handling

Initialization errors are returned to player creation and logged. Drawing checks device removal, framebuffer completeness, interop lock/unlock, resize and presentation results. Minimized windows skip frames. A rendering failure is logged and delivered as a player error, stops the render loop and tells the user to close and reopen the player to retry. Reopening creates fresh resources; automatic device recreation is not implemented. Resize releases the intermediate texture before resizing buffers and recreates it at the new extent.

Ticket 06 device-loss recreation remains an explicit limitation. `Runtime::new` owns and creates the WGL context as well as the D3D11/DirectComposition resources; replacing that runtime would destroy the current GL context retained by mpv. Safe recovery needs independently replaceable D3D/interop resources and validated teardown of a removed device on the same current GL context. The current implementation does not attempt speculative retries.

If hardware validation shows this bottom-target/interop design is a no-go, the fallback is a gpui fork with an external-texture element. That is a contingency, not an implemented or selected alternative. Do not introduce the fork merely because GPU evidence is unavailable.

## libmpv dependency and distribution

The current source of the pin is [`.github/workflows/windows.yml`](../../.github/workflows/windows.yml):

- Upstream: `shinchiro/mpv-winbuild-cmake`, release `20261103`.
- Archive: `mpv-dev-x86_64-20261103-git-a687796d9d.7z`.
- URL: `https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20261103/mpv-dev-x86_64-20261103-git-a687796d9d.7z`.
- SHA256: `41cbfda596aaf1dc30053679bcd22828afaeec411340de6bf4c0e786fed1480b`.

The parent task verified the digest through the GitHub API; it was not independently downloaded or verified for this ADR. The workflow verifies the downloaded archive against the pin. These nightly assets are subject to upstream deletion, so mirroring and release tooling remain follow-up work, not an available guarantee.

The archive supplies a MinGW import library. The workflow instead creates the MSVC `mpv.lib` from the actual DLL exports in an x64 MSVC developer environment, after extracting the checksum-validated archive to `mpv`:

```powershell
$dll = Get-ChildItem mpv -Recurse -Filter libmpv-2.dll | Select-Object -First 1
if (-not $dll) { throw 'libmpv-2.dll not found' }
$names = dumpbin /nologo /exports $dll.FullName |
  Select-String '^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]{8}\s+(\S+)' |
  ForEach-Object { $_.Matches[0].Groups[1].Value }
if (-not $names) { throw 'no exports read from libmpv-2.dll' }
@('LIBRARY libmpv-2', 'EXPORTS') + $names | Set-Content mpv\mpv.def
lib /nologo /def:mpv\mpv.def /out:mpv\mpv.lib /machine:x64
if ($LASTEXITCODE) { throw 'lib failed' }
$env:MPV_LIB_DIR = (Resolve-Path mpv).Path
```

Ship `libmpv-2.dll` beside `tbis.exe`, as the current workflow stages it. That staging is not a completed release process. Distribution must satisfy [ADR-0004](0004-bundle-libmpv-and-license-as-gpl.md): GPL-3.0-or-later licensing, bundled dependency notices/licenses and Corresponding Source for tbis and the exact bundled dependencies. An upstream binary download link alone does not satisfy the source obligation.

## Acceptance evidence still required

See the [Windows validation matrix](../windows.md). Intel, AMD and NVIDIA playback/composition evidence, decoder selection, shader application, screenshots, device-loss recovery by reopening, and window-chrome acceptance are all unverified. Keep this ADR proposed until ticket 05 supplies hardware evidence; do not infer acceptance from compilation or a non-GPU self-test.
