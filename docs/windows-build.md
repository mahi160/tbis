# Windows builds and releases

## Status and installation

The Windows target is x86_64 Windows 10/11 with MSVC. libmpv is built directly from pinned sources on native Windows; no pre-created binary mirror or manually supplied source bundle is needed.

The last recorded native Windows CI run, [42158586879](https://github.com/mahi160/tbis/actions/runs/42158586879) at commit `a35e1a1`, succeeded through packaging and the GPU-free executable self-test. It predates the later build-script, updater and workflow consolidation, so it is not evidence for the current scripts; the next `windows.yml` run on `feat/windows` is. No product or dependency release has been published. Windows UI, login, network playback, GPU interoperability and SMTC/power/cursor behavior on a human PC remain **unverified**; see [Windows validation status](windows.md) for the GPU matrix. A successful self-test is not a playback test.

To try a build, download `windows-release` from **Artifacts** of a successful `windows.yml` run. Extract the artifact container first, then extract the **entire** inner `tbis-<version>-windows-x86_64.zip` into a writable folder and run `tbis.exe`. Keep `libmpv-2.dll`, `LICENSE`, `licenses/`, `components/` and `bundled-components.txt` beside it; do not run from inside the zip or copy only the executable. Keep the matching `tbis-<version>-windows-sources.tar` available when distributing. The application is not Windows-code-signed; review Windows download warnings and verify the release's provenance before running it. GPU drivers remain a system prerequisite.

## Pinned source build

[`scripts/windows-deps.json`](../scripts/windows-deps.json) is the manifest. It holds every source archive (HTTPS URL and SHA256), the three pinned tool archives, the build options, the source layout, and the allowlist of Windows system DLLs and link libraries that the import gate accepts. Current pins include:

- mpv 0.42.0 at `b0841e8c4d156b8403dc1876c4cbd9011eb583f3`.
- FFmpeg 9.0 Meson port at `7cb50518315d038e0f8c585f9ea11195cbea7553` (`meson-9.0` in the manifest).
- libplacebo 7.387.0, libass 0.17.5, dav1d 1.5.4, FreeType 2.15.0, FriBidi 1.0.18, HarfBuzz 13.0.1 and zlib 1.3.2.
- Exact WrapDB, glad, Jinja, MarkupSafe and fast_float revisions for the zlib overlay and libplacebo build inputs.
- Checksum-pinned Meson 1.12.1, Ninja 1.13.2 and NASM 2.16.03 tool archives.

The build is four PowerShell scripts and one Python helper:

- `scripts/windows-common.ps1` loads the manifest and provides the native-call and import-gate helpers.
- `scripts/setup-windows-deps.ps1` runs the recipe and exports `MPV_LIB_DIR`, `MPV_DLL` and `PATH`.
- `scripts/build-windows-mpv.ps1` is the recipe: it prepares sources, records the toolchain, configures Meson, audits the graph, compiles with Ninja, checks the DLL and writes the inventory.
- `scripts/bundle-windows.ps1` packages the application and source archive.
- `scripts/windows_sources.py` (Python 3.12 standard library only) has the subcommands `prepare`, `audit`, `inventory`, `verify`, `toolchain`, `archive` and `checksums`. `toolchain` records tool and SDK identities in `toolchain.json` and generates Meson's `native.ini`; `checksums` writes `SHA256SUMS` for a directory or file.

The recipe uses native Windows x64 GNU-frontend `clang`/`clang++` targeting MSVC, with `lld-link`, `llvm-lib`, `llvm-rc` and the MSVC/Windows SDK environment. It does not use `clang-cl`. All dependency libraries are static, libmpv alone is shared, and Meson requires the static MSVC CRT (`b_vscrt=mt`). Rust's Windows target also uses the static CRT through `.cargo/config.toml`.

Vulkan is disabled in mpv, FFmpeg and libplacebo. libplacebo's public `vulkan.h` unconditionally requires Vulkan-Headers even with its Vulkan backend disabled, so the manifest pins this header-only source at `74d8a6cb930c68ef617b202c3ff3c59d919e086b`. Preparation places it in `libplacebo/3rdparty/Vulkan-Headers` and preserves its archive, licenses and inventory records. The presentation backend uses WGL/OpenGL with D3D11/DirectComposition ([ADR-0007](adr/0007-windows-video-surface.md)); no `vulkan-1.dll` is required. FFmpeg enables D3D11VA, Schannel HTTPS, dav1d, zlib and the required decoding/screenshot features, which the graph audit checks. mpv's own SMTC support is disabled because tbis owns the Windows media-control session.

The build patch [`mpv-d3d11-hwdec-helpers.patch`](../scripts/patches/mpv-d3d11-hwdec-helpers.patch) keeps the D3D11 helpers and feature probes enabled when only hardware decoding is enabled; without it `vf_d3d11vpp.c` fails with a `DXGI_DEBUG_D3D11` redefinition. Preparation applies it strictly with Git before recording prepared-source hashes, and `.gitattributes` keeps build patches LF-terminated so they match extracted upstream sources.

The recipe sets `GIT_CEILING_DIRECTORIES` to the dependency root so source version probes are isolated from the application's Git repository. `windows_sources.py` handles verified downloads and safe extraction: it rejects unsafe paths, collisions and escaping links, and materializes valid internal archive links as files. Preparation creates the pinned subprojects, generated wraps and zlib overlay. Meson uses `--wrap-mode=nodownload` and forced fallbacks, with external pkg-config/CMake discovery disabled. The audit rejects unexpected projects, shared dependencies, external libraries, non-static CRT settings and missing required FFmpeg features. After compiling, the recipe checks the DLL's imports against the manifest allowlist, requires the `mpv_create` and `mpv_render_context_create` exports, and generates MSVC `mpv.lib` from the DLL's actual exports.

Compiler/tool identities, the SDK environment and the runner image are recorded in `toolchain.json`, not pinned. Pinned sources and recipes do **not** make the binaries bit-reproducible or establish legal completeness.

## Build locally

Install stable Rust with the `x86_64-pc-windows-msvc` target and Clippy, PowerShell 7, Python 3.12, Git, Visual Studio C++ Build Tools with the Windows SDK, and LLVM providing `clang`, `clang++`, `lld-link`, `llvm-lib` and `llvm-rc`. Use an x64 Visual Studio developer environment with `lib` and `dumpbin` available. Setup downloads its pinned Meson, Ninja and NASM tools; CMake and 7-Zip are not needed. The application icon is the committed `assets/icon.ico`, embedded by `build.rs`.

From the repository root, in the same PowerShell session:

```powershell
./scripts/setup-windows-deps.ps1
cargo run --release --locked
```

Setup recreates `target/windows-deps`, sets `MPV_LIB_DIR` to `target/windows-deps/mpv` and `MPV_DLL` to that directory's `libmpv-2.dll`, and adds the DLL directory to `PATH`. A fresh session must rerun setup or restore those values. For a dependency-only build, run `./scripts/build-windows-mpv.ps1`.

The checks CI runs are:

```powershell
python -B scripts/windows_sources_tests.py
cargo test --release --locked
cargo clippy --all-targets --locked -- -D warnings
```

The Python tests cover manifest pins, safe extraction, build inputs, notices and inventory integrity; they are not a native compiler or GPU test. `cargo test` includes the unit tests of the in-process updater (`src/update/bundle.rs`). Release cross-compilation from macOS is not possible: gpui's Windows-host-only HLSL generation does not produce `shaders_bytes.rs` there.

## Generated source and provenance contract

`target/windows-deps/corresponding-source/` is generated by the build:

```text
inventory.json
sources/<pinned-source-archives>
licenses/<component>/<upstream-notices>
recipes/scripts/<owned-recipes-and-manifest>
recipes/.github/workflows/<Windows-workflows>
recipes/.github/actions/setup/action.yml
recipes/prepared-source-sha256.json
recipes/generated/<wraps-and-zlib-overlay>
recipes/build/<configuration-logs-and-toolchain-records>
```

`RECIPES` in `scripts/windows_sources.py` defines the preserved inputs: `.gitattributes`, the manifest, the PowerShell scripts, the Python helper and its tests, both Windows workflows, the shared `.github/actions/setup` composite action and the build patch. Build records include Meson introspection and logs, generated configuration headers, Ninja files, compile commands where present, `native.ini`, `toolchain.json`, `build-command.json` and `build.log`. The prepared-source record hashes the assembled source tree before configuration; the graph audit checks it before and after compilation and rejects any source mutation.

The schema-version-1 inventory binds `binarySha256` to the actual `libmpv-2.dll`, `binaries` to all files in the generated `mpv` directory, `manifestSha256` to the current manifest and `recipeInputs` to current `RECIPES` hashes. `files` records every preserved source/recipe/license file. Each component must match the manifest's name, version, URL and source archive hash and have nonempty, exact source/recipe/license record lists.

Verification rehashes the DLL, binary files, manifest, current recipes, preserved recipes and source archives, and compares the complete recorded file set. Missing, changed or mismatched evidence fails closed. Mechanical checks do not replace maintainer review of source and license completeness. To verify an existing local dependency build without rebuilding:

```powershell
python -B scripts/windows_sources.py verify --repo . --deps target/windows-deps
```

## Packaging and source archive rebuild

Package only a clean, committed source revision: the bundle script rejects tracked changes and untracked source inputs and uses `git archive HEAD`. After setup, run:

```powershell
./scripts/bundle-windows.ps1
```

Packaging requires the locally source-built DLL and matching generated provenance. It builds the `dist` profile with `--locked`, stages the flat root the updater expects, and checks the imports of every staged executable and DLL with `dumpbin /dependents`, including delay-load imports. Only bundled files and the manifest's system-DLL allowlist pass; `vcruntime`, `msvcp` and `concrt` imports are forbidden even if bundled. A failed import check, empty parsed imports or unknown dependencies stop packaging; do not bypass or weaken the gates.

The staged executable then runs its GPU-free `--self-test` with its directory on `PATH`. This initializes libmpv and reports its version before UI/GPU initialization; it does not prove playback. Outputs are `target/tbis-<version>-windows-x86_64.zip` and `target/tbis-<version>-windows-sources.tar`. The zip holds `tbis.exe`, `libmpv-2.dll`, `LICENSE`, `licenses/` (including vendored Rust notices) and `components/` (inventory, manifest and `Cargo.lock`) with `bundled-components.txt`.

The source tar contains `sources/tbis/` at the built Git revision (with `cargo vendor --locked --versioned-dirs` output and the matching Cargo configuration), `sources/tbis-revision.txt` and `sources/dependencies/` with the generated dependency source/provenance tree.

To rebuild from a released source archive:

1. Verify the release archive's checksum/provenance, then extract the tar into a fresh directory. Read `sources/tbis-revision.txt` and `sources/dependencies/inventory.json`.
2. Use the prerequisites above. From `sources/tbis`, run `./scripts/setup-windows-deps.ps1`, then `cargo build --profile dist --locked`. The vendored Cargo configuration supplies the locked Rust sources. Setup recreates its work directory and fetches source/tool archives against the preserved manifest hashes; it is not an offline rebuild.
3. Run the `verify` command above against the new dependency build and compare its pins, recipes, prepared-source hashes and build/toolchain records with `sources/dependencies/`. Build records describe the original machine and are not commands to replay. Different compiler/SDK/runner identities can produce different DLL hashes.

The extracted tar is not a Git checkout, so rebuilding the executable does not need the bundle script. Repackaging requires a clean checkout of the recorded revision and the same gates.

## CI and optional dependency publication

`windows.yml` runs on pushes to `feat/windows`, manual dispatch and reuse by `release.yml`. On `windows-2025` it uses the shared `.github/actions/setup` composite (optionally checking out the prepared release revision, then Python 3.12 and the x64 MSVC environment), installs Rust, runs the provenance tests, builds the dependencies through setup, runs `cargo test`, Clippy with `-D warnings`, and bundles through the import gate and packaged self-test. Every run builds from source and uploads `windows-release` (zip and sources tar).

The optional **Build Windows dependencies** workflow (`mirror-windows-deps.yml`) is dispatch-only with `publish=false` by default. It runs the same recipe, then archives `mpv/` and `corresponding-source/` together in `windows-mpv-dependency.tar` with internal and external `SHA256SUMS` (written by `windows_sources.py checksums`). A deliberate `publish=true` run can create the dedicated, immutable `windows-deps-<full Git SHA>` release, never the latest product release and never replacing existing assets. Setup does not consume it.

A `prod` push or deliberate **Release** dispatch prepares one version/revision for both platform builders, transferred as a Git bundle; preparation does not push its version commit or tag. Only after both platforms' tests, bundles and source archives succeed does publication push the prepared commit/tag and create one release with both application zips and source archives, followed by the Homebrew cask update. Windows checksum, source and import failures block the release. Production release publication and update end-to-end on Windows remain untested.
