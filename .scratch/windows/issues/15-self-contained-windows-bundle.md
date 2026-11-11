# 15 — Self-contained Windows bundle (ADR-0004 for Windows)

**What to build:** The Windows zip runs on a clean Windows 10/11 with nothing installed, the same promise as `tbis.app`. Today's artifact ships `tbis.exe` + `libmpv-2.dll` only, and both are missing pieces:

- `libmpv-2.dll` (shinchiro 20261103) hard-imports `vulkan-1.dll` (no delay-load); every other import is a system DLL. Without the Vulkan loader, which GPU drivers normally install, Windows refuses to load libmpv and tbis does not start. Bundle the Khronos Vulkan loader (`vulkan-1.dll`, Apache-2.0) from a pinned, checksummed source.
- Rust's MSVC target links the C runtime dynamically (`VCRUNTIME140.dll`), which a fresh Windows may lack. Link it statically: `-C target-feature=+crt-static` for `x86_64-pc-windows-msvc` in `.cargo/config.toml` (unverified until the built exe's imports are checked).
- `LICENSE`, `licenses/` (mpv, FFmpeg and the static libs shinchiro builds in, plus the Vulkan loader) and `bundled-components.txt`, like `Contents/Resources/` on macOS.
- `scripts/bundle-windows.ps1`, the counterpart of `scripts/bundle.sh`, used by `windows.yml`; it fails if `tbis.exe` or any bundled DLL imports a non-system DLL that isn't in the zip.

**Blocked by:** None — can start immediately. The GPL Corresponding Source for public releases (shinchiro recipe at the pinned commit + every upstream source) belongs to ticket 13.

**Status:** done

- [ ] The import check passes in CI and lists only system DLLs plus bundled ones
- [ ] `tbis.exe` no longer imports `VCRUNTIME140.dll`
- [ ] The zip starts on a Windows install without GPU drivers (CI runner) far enough to run `--self-test`
- [ ] License files for every bundled component are in the zip
