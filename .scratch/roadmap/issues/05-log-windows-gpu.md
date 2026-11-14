# 05 — Log the Windows GPU at Player start

**What to build:** The Windows video path needs an OpenGL driver with `WGL_NV_DX_interop2` on the same adapter as Direct3D. When it fails, the log does not say which GPU or driver was involved. Log the OpenGL vendor, renderer and version, and the Direct3D adapter name, once per Player start.

**Model:** sonnet

**Blocked by:** None.

**Status:** done (runtime unverified, Windows CI not run). `glGetString` comes from the `windows` crate's opengl32 binding (already used for `glGenTextures`), not `load!`, so a missing entry point can't fail the start; the existing `IDXGIDevice`/adapter lookup moved before `wglDXOpenDeviceNV` and is reused for the swap-chain factory. `Runtime::new` now creates the D3D device and logs its adapter before `Gl::new`, so the adapter line exists even when `Gl::new` fails on a missing `WGL_NV_DX_interop2`.

**Read first:** `src/platform/windows/wgl.rs` (all of it: the `load!` macro, `Gl::new`), `src/platform/windows/video.rs` (`Runtime::new`), `docs/adr/0007-windows-video-surface.md`.

## Steps

1. In `wgl.rs`, load `glGetString` the same way the other GL entry points are loaded (`load!`). After the 3.3 core context is current in `Gl::new`, read `GL_VENDOR` (0x1F00), `GL_RENDERER` (0x1F01) and `GL_VERSION` (0x1F02). Treat a null return as `"unknown"`.
2. Write one line: `crate::log::write(&format!("windows video: GL {vendor} | {renderer} | {version}"))`.
3. Log it before the `WGL_NV_DX_interop2` check, so the line exists even when that check fails.
4. In `video.rs`, `Runtime::new`, after `D3D11CreateDevice` succeeds, get the adapter description through `IDXGIDevice::GetAdapter` and `GetDesc`, and write `windows video: D3D adapter {name}`. Add the `windows` crate features this needs to `Cargo.toml` only when the compiler asks for them.
5. When `wglDXOpenDeviceNV` fails, the existing error text stays; the two lines above are now in the log before it.

## Done when

- [x] A Player start on Windows writes one `windows video: GL` line and one `windows video: D3D adapter` line
- [x] Both lines are written before any failure that follows them
- [x] No new `unsafe` block lacks a null check on the pointer it reads

**Check:** dispatch the **Windows** workflow in GitHub Actions; it must pass tests and Clippy. On a Mac, run the gate in `PLAN.md` to prove nothing else broke, and write `Windows CI not run` in the Status when you could not dispatch it.
