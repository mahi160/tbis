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

The pin lives in [`scripts/windows-deps.json`](../../scripts/windows-deps.json), not in the workflow. Normal Windows CI and local setup build it directly through `scripts/build-windows-mpv.ps1`:

- mpv 0.42.0 at `b0841e8c4d156b8403dc1876c4cbd9011eb583f3`.
- FFmpeg 9.0 Meson port at `7cb50518315d038e0f8c585f9ea11195cbea7553`.
- libplacebo 7.387.0, libass 0.17.5 and all remaining dependency/build inputs pinned by source URL and SHA256 in the manifest.
- Meson 1.12.1, Ninja 1.13.2 and NASM 2.16.03 checksum-pinned.

The native Windows recipe uses GNU-frontend Clang targeting MSVC and the Windows SDK. All dependency libraries and the CRT are static; only libmpv is shared. Vulkan is disabled in mpv, FFmpeg and libplacebo. libplacebo's public `vulkan.h` still unconditionally requires Vulkan-Headers, so the manifest pins this header-only source at `74d8a6cb930c68ef617b202c3ff3c59d919e086b` and preparation installs it under `libplacebo/3rdparty/Vulkan-Headers`, preserving its original archive, licenses and inventory records. This build-only header dependency does not enable the Vulkan backend or require a Vulkan loader binary (`vulkan-1.dll`). The recipe generates MSVC `mpv.lib` from the built DLL's actual exports and verifies client/render exports and imports.

Python 3.12 standard-library tooling verifies source downloads/extraction and generates source/license/provenance records tied to the actual DLL hash, current scripts and manifest pins. No pre-created mirror, manual source URL/SHA variables or supplied reviewer attestation is required. Compiler/SDK/runner identities are recorded, not fully pinned for bit-for-bit reproducibility.

Ship `libmpv-2.dll` beside `tbis.exe` with the licenses and component records in the flat updater-compatible zip; the updater is unchanged. Distribution must satisfy [ADR-0004](0004-bundle-libmpv-and-license-as-gpl.md). Checksum, matching-source and PE import gates remain fail-closed. The optional **Build Windows dependencies** dispatch defaults to `publish=false`; later explicit publication can create a dedicated immutable matched binary/source/provenance archive, never the latest application release. Normal CI builds from source without it.

This session configured, audited and cross-compiled the entire pinned Windows source graph on macOS with the actual MSVC SDK and GNU-frontend LLVM 23.1.4, linking `mpv-2.dll` in 2,979 steps. The actual helper generated and verified its paired source inventory under `target/windows-source-final-validated`; mechanical inventory validation does not establish legal completeness. The DLL imports only Windows system libraries, with no Vulkan or Visual C++ runtime DLL imports, and exposes all 54 mpv client/render exports. All 20 Python tests, 30 Rust release tests, six PowerShell script parses, portable updater rollback tests and three workflow YAML parses passed.

Cross-validation used native Mac Ninja 1.13.2 and NASM 3.02 rather than the native recipe's pinned NASM 2.16.03. The exact native PowerShell recipe, native CI, packaging and executable self-test remain unproven; no new CI was pushed or dispatched and no product or dependency release was published. Earlier Windows application cross-debug-link and Clippy evidence used the old nightly and remains historical, not validation against this DLL. See [Windows build instructions](../windows-build.md#source-pipeline-validation) for commands and limits. Compilation does not prove playback or GPU acceptance.

## Acceptance evidence still required

See the [Windows validation matrix](../windows.md). Intel, AMD and NVIDIA playback/composition evidence, decoder selection, shader application, screenshots, device-loss recovery by reopening, and window-chrome acceptance are all unverified. Keep this ADR proposed until ticket 05 supplies hardware evidence; do not infer acceptance from compilation or a non-GPU self-test.
