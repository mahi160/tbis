# Windows builds and releases

## Status and installation

The Windows target is x86_64 Windows 10/11 with MSVC. libmpv comes from a pinned, checksummed prebuilt package; CI does not compile mpv or FFmpeg.

The switch to the prebuilt package has not had a native Windows CI run yet; the next `windows.yml` run on `feat/windows` is the evidence for the current scripts. Windows UI, login, network playback, GPU interoperability and SMTC/power/cursor behavior on a human PC remain **unverified**; see [Windows validation status](windows.md) for the GPU matrix. A successful self-test is not a playback test.

To try a build, download `windows-release` from **Artifacts** of a successful `windows.yml` run. Extract the artifact container first, then extract the **entire** inner `tbis-<version>-windows-x86_64.zip` into a writable folder and run `tbis.exe`. Keep `libmpv-2.dll`, `vulkan-1.dll`, `LICENSE`, `licenses/`, `components/` and `bundled-components.txt` beside it; do not run from inside the zip or copy only the executable. The application is not Windows-code-signed; review Windows download warnings before running it. GPU drivers remain a system prerequisite.

## Pinned dependencies

[`scripts/windows-deps.json`](../scripts/windows-deps.json) is the manifest:

- `libmpv`: shinchiro's [mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake) `mpv-dev-x86_64` package (release tag, file name, SHA-256, the mpv revision it contains, and the build run). shinchiro builds mpv, FFmpeg and their libraries with MinGW/clang and links them statically into one `libmpv-2.dll`, with Lua and Vulkan enabled.
- `vulkanLoader`: the Khronos Vulkan-Loader and Vulkan-Headers tag and source archive hashes. The package's `libmpv-2.dll` hard-imports `vulkan-1.dll`, which Windows only has when a GPU driver installed it, so tbis builds and ships the loader beside the DLL, as upstream mpv's Windows builds do.
- `systemDlls` and `systemDllPrefixes`: the system DLLs the import gate accepts.

shinchiro keeps only about 30 days of releases. The **Mirror Windows libmpv** workflow (`mirror-windows-deps.yml`, dispatch only) copies the pinned package to a `libmpv-<release>` release of this repository after checking its hash. Setup downloads from that mirror first and falls back to shinchiro.

To move the pin: pick a new shinchiro release, update `release`, `file`, `sha256`, `mpv`, `mpvCommit`, `workflowRun`, `mirror` and `upstream`, run the mirror workflow, then run `windows.yml`.

## Build locally

Install stable Rust with the `x86_64-pc-windows-msvc` target and Clippy, PowerShell 7, Git, 7-Zip (`7z` on `PATH`), CMake, and Visual Studio C++ Build Tools with the Windows SDK. Use an x64 Visual Studio developer environment with `lib` and `dumpbin` available. The application icon is the committed `assets/icon.ico`, embedded by `build.rs`.

From the repository root, in the same PowerShell session:

```powershell
./scripts/setup-windows-deps.ps1
cargo run --release --locked
```

Setup recreates `target/windows-deps`, then:

1. downloads the libmpv package, checks its SHA-256 and extracts `libmpv-2.dll` and its headers into `target/windows-deps/mpv`;
2. requires the `mpv_create` and `mpv_render_context_create` exports and generates MSVC `mpv.lib` from the DLL's exports (`dumpbin`, `lib /def`);
3. downloads the Vulkan-Headers and Vulkan-Loader archives, checks their hashes, and builds `vulkan-1.dll` with CMake (static CRT, `UPDATE_DEPS=OFF` so nothing unpinned is fetched), checking it is x64;
4. checks both DLLs' imports against the manifest allowlist;
5. sets `MPV_LIB_DIR` to `target/windows-deps/mpv`, `MPV_DLL` to its `libmpv-2.dll`, and adds that directory to `PATH`.

A fresh session must rerun setup or restore those values.

The checks CI runs are:

```powershell
cargo test --release --locked
cargo clippy --all-targets --locked -- -D warnings
```

`cargo test` includes the unit tests of the in-process updater (`src/update/bundle.rs`). Release cross-compilation from macOS is not possible: gpui's Windows-host-only HLSL generation does not produce `shaders_bytes.rs` there.

## Packaging

Package only a clean, committed source revision: the bundle script rejects tracked changes and untracked source inputs, because the source offer names the built Git revision. After setup, run:

```powershell
./scripts/bundle-windows.ps1
```

It builds the `dist` profile with `--locked`, stages the flat root the updater expects, and checks the imports of every staged executable and DLL with `dumpbin /dependents`, including delay-load imports. Only bundled files and the manifest's system-DLL allowlist pass; `vcruntime`, `msvcp` and `concrt` imports are forbidden even if bundled. A failed import check, empty parsed imports or unknown dependencies stop packaging; do not bypass or weaken the gates.

The staged executable then runs its GPU-free `--self-test` with its directory on `PATH`. This initializes libmpv and reports its version before UI/GPU initialization; it does not prove playback.

The output is `target/tbis-<version>-windows-x86_64.zip`, holding `tbis.exe`, `libmpv-2.dll`, `vulkan-1.dll`, `LICENSE`, `bundled-components.txt`, `licenses/` (the Vulkan loader's license and the vendored Rust crates' notices) and `components/` (`windows-deps.json`, `Cargo.lock` and `sources.txt`).

## Source offer

`components/sources.txt` is the GPL source offer for the Windows build ([ADR-0004](adr/0004-bundle-libmpv-and-license-as-gpl.md)). It names the tbis Git revision, the libmpv package with its hash, mirror and upstream URLs, the mpv revision, shinchiro's build recipe (which lists FFmpeg and every other library with its source URL) and build run, and the Vulkan loader source archives with their hashes. No source archive is attached to Windows releases; the offer points to the upstream sources.

## CI and releases

`windows.yml` runs on pushes to `feat/windows`, manual dispatch and reuse by `release.yml`. On `windows-2025` it uses the shared `.github/actions/setup` composite (optionally checking out the prepared release revision, then the x64 MSVC environment), installs Rust, runs setup, `cargo test`, Clippy with `-D warnings`, and bundles through the import gate and packaged self-test. It uploads `windows-release` (the zip).

A `prod` push or deliberate **Release** dispatch prepares one version/revision for both platform builders, transferred as a Git bundle; preparation does not push its version commit or tag. Only after both platforms' tests and bundles succeed does publication push the prepared commit/tag and create one release with both application zips and the macOS source archive, followed by the Homebrew cask update. Windows checksum and import failures block the release. Production release publication and update end-to-end on Windows remain untested.
