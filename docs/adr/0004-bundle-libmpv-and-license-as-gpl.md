---
status: accepted
---

# Bundle libmpv in the app and license tbis as GPL-3.0-or-later

This supersedes ADR-0002. tbis is meant for non-technical users as well as technical ones, and asking people to install Homebrew and mpv before the app will even launch rules out the first group. `scripts/bundle.sh` therefore copies Homebrew's libmpv, the `mpv` binary that PiP spawns (ADR-0003), and every Homebrew dylib either of them loads into `tbis.app/Contents/Frameworks` and `Contents/MacOS`. It relinks all of them to `@rpath` and fails the build if any reference into the Homebrew prefix remains. Building still needs `brew install mpv`; running the app does not.

Homebrew's mpv and FFmpeg are built with `--enable-gpl --enable-version3`, so an app bundle containing them can only be distributed under GPLv3 or later. tbis is licensed GPL-3.0-or-later (`LICENSE`, `Cargo.toml`). This is the same trade-off Jellyfin Media Player (GPL-2.0), IINA (GPL-3.0) and Moonfin (GPL-2.0) made.

## Considered Options

- **Keep libmpv an external dependency (ADR-0002)**, with a friendlier missing-mpv screen and a Homebrew cask. This was rejected because it still requires Homebrew, which non-technical users won't have.
- **Bundle an LGPL-only libmpv** (for example media-kit's `libmpv-darwin-build`) and keep tbis permissively licensed. This was rejected for now. Those builds ship libmpv but no `mpv` binary, so PiP would need to be rebuilt another way. GPL-only features would be lost, and a second libmpv build pipeline would have to be maintained. Revisit this if tbis ever needs to be closed-source or ship in the Mac App Store.

## Consequences

- The app is about 100 MB instead of a few MB, and contains 48 bundled dylibs.
- Each bundled formula's license files are copied to `Contents/Resources/licenses/<formula>-<version>/`, and the list of bundled components is written to `Contents/Resources/bundled-components.txt`.
- **Source obligation:** every distributed build must come with the Corresponding Source (GPLv3 section 6). That means tbis's own source and the source of each bundled formula at the listed version, for example as source archives attached to the same release. Upstream download links alone are not enough.
- Libraries and files that the bundled code loads at runtime rather than links, such as a Vulkan driver, are not bundled. PiP's standalone mpv is expected to fall back to its OpenGL output on a Mac without MoltenVK; this has not been verified.
- Distribution to other people still needs a Developer ID signature and notarization so Gatekeeper opens the app. The bundle is only ad-hoc signed for now. Auto-update is not built yet.
- The app stays out of the Mac App Store, because the GPL is incompatible with its terms.

## Windows addition

Windows local builds, normal CI and production releases use the same owned source recipe, `scripts/build-windows-mpv.ps1`, through `scripts/setup-windows-deps.ps1`. The checksum-pinned source manifest is `scripts/windows-deps.json`: mpv 0.42.0 at `b0841e8c4d156b8403dc1876c4cbd9011eb583f3`, FFmpeg's 9.0 Meson port at `7cb50518315d038e0f8c585f9ea11195cbea7553`, libplacebo 7.387.0, libass 0.17.5 and their pinned dependency/build inputs. No pre-created mirror, expiring nightly, supplied source bundle, `WINDOWS_SOURCE_URL`/`WINDOWS_SOURCE_SHA256` variables or supplied `reviewedBy` attestation is required.

Python 3.12 standard-library tooling verifies downloads and safe extraction, preserves source archives and notices, and generates an inventory bound to the actual built DLL hash, all binary files, current manifest and recipe inputs. Preserved recipes include scripts, workflows, generated wraps/overlay, prepared-source hashes and build/configuration/toolchain records. Verification rehashes the complete file set and checks every component against its source pin. Missing or mismatched evidence blocks packaging and public distribution; mechanical checks do not replace maintainer review of Corresponding Source and license completeness.

Native Windows GNU-frontend Clang targets MSVC with the Windows SDK. Meson 1.12.1, Ninja 1.13.2 and NASM 2.16.03 are checksum-pinned. Dependency libraries and their MSVC CRT are static; only libmpv is shared. Rust also uses the static CRT. Compiler, SDK and runner identities are recorded but not fully pinned, so this is not a claim of bit-for-bit reproducibility.

The Windows zip has a flat updater-compatible root: `tbis.exe`, `libmpv-2.dll`, `LICENSE`, `licenses/`, `components/` and `bundled-components.txt`. Vulkan is disabled across mpv, FFmpeg and libplacebo; the backend uses WGL/OpenGL and D3D11/DirectComposition, with no `vulkan-1.dll` requirement. Packaging inspects every staged PE's imports, rejects unbundled non-system dependencies and Visual C++ runtime DLLs, and fails on unsuccessful or empty import inspection. The staged executable must pass its existing GPU-free `--self-test`; that is not GPU acceptance evidence. The updater is unchanged.

Windows licenses and inventories travel in the application zip, including notices from vendored Rust crates. The separate Windows source archive contains tbis at the built revision, locked vendored Rust dependencies, and the generated dependency sources, licenses, recipes and provenance. Both platform builds use one prepared version/revision; publishing the version commit, tag, release assets and subsequent cask update waits for both platforms' tests, bundles and source archives.

The optional **Build Windows dependencies** dispatch defaults to `publish=false`. A later explicit publication can create a dedicated immutable `windows-deps-<full Git SHA>` release containing matched binaries, sources, provenance and checksums, never the latest product release and never replacing existing assets. Normal builds do not depend on that publication.

This session configured, audited and cross-compiled the entire pinned Windows source graph on macOS with the actual MSVC SDK and GNU-frontend LLVM 23.1.4, linking `mpv-2.dll` in 2,979 steps. The actual helper generated and verified its paired source inventory under `target/windows-source-final-validated`; this proves mechanical consistency, not legal completeness. The DLL has only Windows system imports, no Vulkan or Visual C++ runtime DLL imports, and all 54 mpv client/render exports. Build-only Vulkan-Headers source remains preserved. All 20 Python tests, 30 Rust release tests, six PowerShell script parses, portable updater rollback tests and three workflow YAML parses passed.

Cross-validation used native Mac Ninja 1.13.2 and NASM 3.02; the default native recipe pins NASM 2.16.03. The exact native PowerShell recipe, native CI, packaging, executable self-test and playback remain unproven. No new CI was pushed or dispatched and no product or dependency release was published. Earlier application cross-build/Clippy evidence used the old nightly, not this DLL; Windows runtime and GPU acceptance remain unverified. See [Windows builds and releases](../windows-build.md#source-pipeline-validation) for commands, provenance limits and separate historical evidence.
