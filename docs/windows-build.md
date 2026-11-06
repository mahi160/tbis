# Windows builds and releases

## Status and installation

The Windows target is x86_64 Windows 10/11 with MSVC. The implemented dependency pipeline builds libmpv directly from pinned sources on native Windows; it no longer requires a pre-created binary mirror or manually supplied source bundle. The entire pinned source graph has been configured, audited and cross-compiled on macOS, linking the Windows DLL and validating its paired source inventory. Native Windows CI, packaging, executable self-test and playback remain pending; Windows runtime and GPU behavior remain **unverified**. See [Windows validation status](windows.md) for the GPU matrix. A successful dependency self-test is not a playback test.

Once a release includes `tbis-<version>-windows-x86_64.zip`, download that asset from [Releases](https://github.com/mahi160/tbis/releases), extract the **entire** zip into a writable folder, and run `tbis.exe`. Keep `libmpv-2.dll`, `LICENSE`, `licenses/`, `components/` and `bundled-components.txt` beside it. Do not run from inside the zip or copy only the executable. The application is not Windows-code-signed; review Windows download warnings and verify the release's provenance before running it. GPU drivers remain a system prerequisite.

## Source pipeline validation

This session's evidence on macOS covers the actual source helper and the entire pinned Windows-target dependency graph:

- `prepare` downloaded, SHA256-validated and safely extracted all 15 pinned source components and three pinned tools. All 20 Python provenance/manifest/archive tests passed on Python 3.15.2; the helper also supports CI's Python 3.12.
- The full Rust release suite passed all 30 tests. All six PowerShell scripts parsed, the existing portable updater rollback tests passed, and all three workflow YAML files parsed. These are not native Windows runtime results.
- Configuration and the actual helper's graph audit passed with the MSVC/Windows SDK and GNU-frontend LLVM 23.1.4. Native Mac Ninja 1.13.2 and NASM 3.02 completed all 2,979 build steps, linking `mpv-2.dll`. The dependency tree is `target/windows-source-final-validated`; its normalized DLL/import library are in `target/windows-source-final-validated/mpv`.
- The built DLL imported only Windows system libraries: no `vulkan-1.dll`, `vcruntime`, `msvcp` or `concrt`. All 54 mpv client/render exports were present. Vulkan-Headers remains preserved as a build-only source dependency, not a Vulkan runtime requirement.
- The actual helper generated and verified the paired `corresponding-source/inventory.json`, binding sources, recipes and notices to the built binaries. This validates the mechanical inventory, not legal completeness.
- Generated metadata reports `FFMPEG_VERSION` as `9.0.2`; libass does not inherit tbis's parent Git identity. `GIT_CEILING_DIRECTORIES` isolates source probes through configuration and compilation. Standards review found no issues; the Spec review's CRLF-patch and parent-Git-version findings were fixed with LF patch attributes and the Git ceiling. The patch attribute disables only blank-at-EOL whitespace checking for valid unified-diff context lines.

The helper invocations and Ninja compilation used these existing contracts (paths are parameterized):

```sh
python -B scripts/windows_sources.py prepare --repo "$REPO" --deps "$DEPS"
python -B scripts/windows_sources.py audit --repo "$REPO" --deps "$DEPS"
ninja -C "$DEPS/build"
python -B scripts/windows_sources.py inventory --repo "$REPO" --deps "$DEPS"
python -B scripts/windows_sources.py verify --repo "$REPO" --deps "$DEPS"
```

`REPO` is the absolute repository root; `DEPS` was its `target/windows-source-final-validated` directory. `python` and `ninja` stand for the validation executables above. `prepare` requires a fresh dependency directory; `audit` requires the configured Meson graph and SDK environment. Configuration occurred between `prepare` and `audit`; DLL/import-library normalization and toolchain/build records preceded `inventory`. These helper actions are the same contracts used by the native recipe, not a standalone build command sequence. The temporary cross file and Mac-only helper tools were validation scaffolding, not a newly supported repository cross-build CLI.

The default native PowerShell recipe uses pinned NASM 2.16.03, not the cross-validation host's NASM 3.02. This evidence does not prove the exact native PowerShell recipe, native CI, packaging, executable self-test or playback. No new CI was pushed or dispatched, and no product or dependency release was published. Native Windows CI remains pending; the GPU matrix remains unverified.

## Recorded validation evidence

The following is historical application validation on an Apple Silicon macOS host, not validation of the new source-built dependency pipeline. Temporary tools were used without a global installation. Commands omit log redirection and use tool names on `PATH` rather than temporary installation paths.

- `cargo test --release --locked`: 30 tests passed on macOS.
- `cargo check --locked`, `cargo clippy --all-targets --locked -- -D warnings` and `cargo fmt --all -- --check`: passed on macOS.
- `pwsh -NoLogo -NoProfile -File src/platform/windows/update.tests.ps1`: portable updater tests passed using PowerShell 7.6.9 for macOS arm64, not Windows process handling or runtime behavior.
- PowerShell parsing passed for the five scripts then present in `scripts/*.ps1` and `src/platform/windows/*.ps1`, using `[Management.Automation.Language.Parser]::ParseFile`. Workflow YAML also parsed with Ruby. These results do not cover the newly added recipe or native CI execution.

Windows cross-compilation used cargo-xwin with the actual MSVC CRT/Windows SDK, LLVM 23.1.4 (`clang-cl`, `lld-link`, `llvm-lib` and `llvm-readobj`) and the repository's static-CRT target configuration. `DYLD_FALLBACK_LIBRARY_PATH` included temporary Z3 5.3.3 and LLVM libraries and Homebrew's zstd library directory. `MPV_LIB_DIR` contained `mpv.lib`, generated with `llvm-lib /def:mpv.def /out:mpv.lib /machine:x64` from the previous shinchiro `20261103` nightly DLL, not the new source build. `TBIS_WINDOWS_ICON` named the generated `icon.ico`. These commands succeeded:

```sh
cargo-xwin check --target x86_64-pc-windows-msvc --locked
cargo-xwin clippy --target x86_64-pc-windows-msvc --all-targets --locked -- -D warnings
cargo-xwin build --target x86_64-pc-windows-msvc --locked
```

The resulting `target/x86_64-pc-windows-msvc/debug/tbis.exe` was linked, not executed. This is debug application evidence only. `cargo-xwin build --release --target x86_64-pc-windows-msvc --locked` failed in `gpui-pre-windows 0.3.9` because `shaders_bytes.rs` was missing: HLSL generation runs only on a Windows host. No shader stubs, dependency patches or weakened release settings hid this limitation.

`llvm-readobj --coff-imports` inspected the old nightly DLL and debug executable; neither imported `vcruntime`, `msvcp` or `concrt`. The nightly archive matched its then-pinned SHA256. This is not import evidence for the new source-built DLL and does not replace native `dumpbin /dependents` packaging checks.

Old Windows CI run `41985341022` failed downloading the nonexistent `windows-deps-20261103` mirror. That failure belongs to the replaced bootstrap, not the new pipeline. It does not describe execution of the new pipeline; current source-pipeline evidence and remaining native CI work are recorded above.

## Pinned source build

[`scripts/windows-deps.json`](../scripts/windows-deps.json) is the source and build-option manifest. Every source archive has an HTTPS URL and SHA256. Current pins include:

- mpv 0.42.0 at `b0841e8c4d156b8403dc1876c4cbd9011eb583f3`.
- FFmpeg 9.0 Meson port at `7cb50518315d038e0f8c585f9ea11195cbea7553` (`meson-9.0` in the manifest).
- libplacebo 7.387.0, libass 0.17.5, dav1d 1.5.4, FreeType 2.15.0, FriBidi 1.0.18, HarfBuzz 13.0.1 and zlib 1.3.2.
- Exact WrapDB, glad, Jinja, MarkupSafe and fast_float revisions for the zlib overlay and libplacebo build inputs.
- Checksum-pinned Meson 1.12.1, Ninja 1.13.2 and NASM 2.16.03 tool archives.

`scripts/build-windows-mpv.ps1` uses native Windows x64 GNU-frontend `clang`/`clang++` targeting MSVC, with `lld-link`, `llvm-lib`, `llvm-rc` and the MSVC/Windows SDK environment. It does not use `clang-cl` for this recipe. All dependency libraries are static, libmpv alone is shared, and Meson requires the static MSVC CRT (`b_vscrt=mt`). Rust's Windows target also uses the static CRT through `.cargo/config.toml`.

Vulkan is disabled in mpv, FFmpeg and libplacebo. libplacebo's public `vulkan.h` unconditionally requires Vulkan-Headers even with its Vulkan backend disabled. The manifest pins this header-only source at `74d8a6cb930c68ef617b202c3ff3c59d919e086b`; preparation places it in `libplacebo/3rdparty/Vulkan-Headers` and preserves its original archive, licenses and inventory records. The Windows presentation backend uses WGL/OpenGL with D3D11/DirectComposition; no Vulkan loader binary (`vulkan-1.dll`) or Vulkan SDK loader build is required. FFmpeg enables D3D11VA, Schannel HTTPS, dav1d, zlib and the required decoding/screenshot features, which the graph audit checks. mpv's own SMTC support is disabled because tbis owns the Windows media-control session.

Our build patch [`mpv-d3d11-hwdec-helpers.patch`](../scripts/patches/mpv-d3d11-hwdec-helpers.patch) ensures D3D11 helpers and feature probes are enabled when only hardware decoding is enabled, preserving the existing D3D11/EGL/Vulkan conditions. An earlier cross-compilation attempt reached 2975/2978 before `vf_d3d11vpp.c` failed with a `DXGI_DEBUG_D3D11` redefinition because the probe was omitted; the patched graph subsequently completed all 2,979 steps. Preparation applies the patch strictly with Git before recording prepared-source hashes; the original archive and exact patch are preserved in the source inventory. Native compilation remains unverified. `.gitattributes` keeps build patches LF-terminated on Windows so they match extracted upstream sources.

The source builder sets `GIT_CEILING_DIRECTORIES` to its resolved dependency root before preparation and retains it through Meson configuration and Ninja compilation. Dependency source version probes are isolated from the parent application Git repository and use preserved upstream release metadata or git-archive substitutions instead.

Python 3.12's standard library in `scripts/windows_sources.py` handles verified downloads, safe extraction and provenance. Extraction rejects unsafe paths, collisions and escaping links, and materializes valid internal archive links as files. Preparation creates the pinned subprojects, generated wraps and zlib overlay. Meson uses `--wrap-mode=nodownload` and forced fallbacks; external pkg-config/CMake discovery is disabled. The audit rejects unexpected projects, shared dependencies, external libraries, non-static CRT settings and missing required FFmpeg features. The recipe checks the DLL's imports and client/render exports before generating MSVC `mpv.lib` from its actual exports.

Compiler/tool hashes and identities, SDK environment and runner image identity are recorded, not fully pinned for bit-for-bit reproducibility. Pinned sources and recipes do **not** establish bit-reproducible binaries, legal completeness or successful native compilation.

## Build locally

Install stable Rust with the `x86_64-pc-windows-msvc` target and Clippy, PowerShell 7, Python 3.12, Git, Visual Studio C++ Build Tools with the Windows SDK, and LLVM providing `clang`, `clang++`, `lld-link`, `llvm-lib` and `llvm-rc`. Use an x64 Visual Studio developer environment with `lib` and `dumpbin` available. Setup downloads its pinned Meson, Ninja and NASM tools; it does not require CMake or 7-Zip for dependency extraction.

From the repository root, in the same PowerShell session:

```powershell
./scripts/setup-windows-deps.ps1
cargo run --release --locked
```

Setup invokes the source recipe, recreating `target/windows-deps`, then generates `target/windows-deps/icon.ico` from `assets/icon.png`. It sets `MPV_LIB_DIR` to `target/windows-deps/mpv`, `MPV_DLL` to that directory's `libmpv-2.dll`, and `TBIS_WINDOWS_ICON` to the icon, and adds the DLL directory to `PATH`. CI exports the same values for later steps. A fresh session must rerun setup or restore those values and paths.

For a dependency-only build, use the owned recipe directly:

```powershell
./scripts/build-windows-mpv.ps1
```

The approved provenance unittest command, also used by Windows CI, is:

```powershell
python -B scripts/windows_sources_tests.py
```

It covers manifest pins, safe extraction, notices and inventory integrity; it is not a native compiler or GPU test. The existing application and updater checks are:

```powershell
cargo test --release --locked
cargo clippy --all-targets --locked -- -D warnings
./src/platform/windows/update.tests.ps1
```

These commands are instructions, not new passing results.

## Generated source and provenance contract

`target/windows-deps/corresponding-source/` is generated by the build, not supplied with a `reviewedBy` attestation:

```text
inventory.json
sources/<pinned-source-archives>
licenses/<component>/<upstream-notices>
recipes/scripts/<owned-recipes-and-manifest>
recipes/.github/workflows/<Windows-workflows>
recipes/prepared-source-sha256.json
recipes/generated/<wraps-and-zlib-overlay>
recipes/build/<configuration-logs-and-toolchain-records>
```

`RECIPES` in `scripts/windows_sources.py` defines the preserved inputs: `.gitattributes`, the manifest, common/setup/build/bundle PowerShell scripts, Python helper and unittest file, and both Windows workflows. Build records include Meson introspection and logs, generated configuration headers, Ninja files, compile commands where present, `native.ini`, `toolchain.json`, `build-command.json` and `build.log`. The prepared-source record hashes the assembled source tree before configuration; the graph audit checks it before and after compilation and rejects any source mutation.

The schema-version-1 inventory binds `binarySha256` to the actual `libmpv-2.dll`, `binaries` to all files in the generated `mpv` directory, `manifestSha256` to the current manifest and `recipeInputs` to current `RECIPES` hashes. `files` records every preserved source/recipe/license file. Each component must match the manifest's name, version, URL and source archive hash and have nonempty, exact source/recipe/license record lists.

Verification rehashes the DLL, binary files, manifest, current recipes, preserved recipes and source archives, and compares the complete recorded file set. Missing, changed or mismatched evidence fails closed. No `WINDOWS_SOURCE_URL` or `WINDOWS_SOURCE_SHA256` variables are required. Mechanical provenance checks do not replace maintainer review of source/license completeness.

To verify an existing local dependency build without rebuilding:

```powershell
python -B scripts/windows_sources.py verify --repo . --deps target/windows-deps
```

## Packaging and source archive rebuild

Package only a clean, committed source revision: the bundle script rejects tracked changes and untracked source inputs and uses `git archive HEAD`. After setup, run:

```powershell
./scripts/bundle-windows.ps1
```

Packaging requires the locally source-built DLL and matching generated provenance. It builds the `dist` profile with `--locked`, stages the flat updater-compatible root and inspects every staged executable/DLL with `dumpbin /dependents`, including delay-load imports. Only bundled dependencies and explicitly allowed Windows system imports pass; `vcruntime`, `msvcp` and `concrt` imports are forbidden even if bundled. Failed inspection, empty parsed imports or unknown dependencies stop packaging; do not bypass or weaken the gates.

The staged executable runs its existing GPU-free `--self-test` with its directory on `PATH`. This initializes libmpv and reports its version before UI/GPU initialization; it does not prove playback. Outputs are `target/tbis-<version>-windows-x86_64.zip` and `target/tbis-<version>-windows-sources.tar`. The zip includes dependency licenses/inventory, manifest, Cargo.lock and vendored Rust notices. Its flat layout and the updater are unchanged.

The source tar contains `sources/tbis/` at the built Git revision, `sources/tbis-revision.txt` and `sources/dependencies/` with the full generated dependency source/provenance tree. The tbis tree includes `cargo vendor --locked --versioned-dirs` output and the corresponding Cargo configuration.

To rebuild from a released source archive:

1. Verify the release archive's checksum/provenance, then extract the tar into a fresh directory. Read `sources/tbis-revision.txt` and `sources/dependencies/inventory.json`; retain the original records for comparison.
2. Use the native Windows prerequisites above. From `sources/tbis`, run `./scripts/setup-windows-deps.ps1`, then `cargo build --profile dist --locked`. The vendored Cargo configuration supplies the locked Rust sources. Setup recreates its work directory and fetches source/tool archives against the preserved manifest hashes; it is not an offline rebuild command.
3. Run the Python `verify` command above against the new dependency build. Compare its pins, recipes, prepared-source hashes and build/toolchain records with `sources/dependencies/`. The original pinned source archives and generated wraps/overlay are preserved there for inspection; build records describe the original machine and are not portable commands to replay blindly. Different compiler/SDK/runner identities can produce different DLL hashes.

The extracted tar is not a Git checkout, so rebuilding the executable does not require running the bundle script. Repackaging requires a clean checkout of the recorded revision and the same source/provenance gates; no integrity bypass is supported.

## CI and optional dependency publication

`windows.yml` supports manual runs, pushes to `feat/windows` and reuse by `release.yml`. On `windows-2025`, it sets up Python 3.12 and the x64 MSVC environment, runs provenance unittests, builds dependencies directly through setup, runs release tests, warning-denying Clippy and updater tests, then bundles through the source/import gates and packaged self-test. Every normal run builds from source; it does not consume a dependency mirror.

The optional **Build Windows dependencies** workflow (`mirror-windows-deps.yml`) is dispatch-only with `publish=false` by default. It runs the same dependency recipe and verification, then archives `mpv/` and `corresponding-source/` together in `windows-mpv-dependency.tar` with internal and external `SHA256SUMS`. Both tar artifacts use Python's standard-library PAX archive creation; the optional dependency archive has an explicit `windows-mpv-dependency/` root. A later deliberate `publish=true` run can create the dedicated `windows-deps-<full Git SHA>` release, never the latest product release. It does not replace existing release assets; treat the matched binary/source/provenance archive as immutable. This is optional publication, not setup bootstrap.

A `prod` push or deliberate **Release** dispatch prepares one version/revision for both platform builders. A Git bundle transfers the prepared revision; preparation does not push its version commit or tag. Only after both platforms' tests, bundles and GPL source archives succeed does publication push the prepared commit/tag and create one release with both application zips and source archives, followed by the existing Homebrew cask update. Windows checksum/source/import failures block public application release. Central tests and source-helper inventory validation passed as recorded above; native CI and end-to-end release wiring remain unproven. Runtime/GPU acceptance is separate and still unverified.
