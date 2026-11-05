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

Windows uses one shared dependency setup script and checksum pin manifest for local builds, Windows CI and production releases. Normal setup downloads libmpv only from the durable repository dependency mirror. The mirror does not exist yet; provisioning is an explicit maintainer action, not an implicit fallback to an expiring nightly. The pinned upstream build's published logs and workflow do not establish complete exact Corresponding Source. Neither the mirror workflow nor application packaging may distribute that binary without a reviewed, checksum-verified matching source/license bundle. If matching source cannot be recovered, this pin cannot be distributed.

The supplied inventory must identify every component incorporated into libmpv, including FFmpeg and static dependencies, with exact sources, patches, build recipes, configurations and license notices. The source gate requires a completeness attestation, named reviewer, matching binary checksum and individually hashed source/recipe/license files. These mechanical validations do not replace the maintainer's completeness and reproducibility review. Upstream links alone remain insufficient. Missing evidence blocks the public mirror and application release; compilation and tests can run before the packaging gate.

The Windows zip has a flat updater-compatible root: `tbis.exe`, `libmpv-2.dll`, `vulkan-1.dll`, `LICENSE`, `licenses/`, `components/` and `bundled-components.txt`. Rust uses the static MSVC CRT. The Vulkan loader is built with its upstream static MSVC runtime setting from checksum-pinned Loader and Headers source archives, not an unpinned SDK redistributable. Packaging inspects every staged PE's imports, rejects unbundled non-system dependencies and Visual C++ runtime DLLs, and fails if dumpbin cannot inspect a file. The staged executable must pass its existing GPU-free `--self-test`; that is not GPU acceptance evidence.

Windows licenses and inventories travel in the application zip. A separate Windows source archive on the same release contains tbis at the built revision, vendored locked Rust dependencies, exact supplied dependency sources/recipes/licenses, and the pinned Vulkan source archives with their notices. Application packaging copies the supplied and Vulkan license notices, plus notices from vendored Rust crates. Both platform builds use one prepared version/revision, and publishing the version commit, tag, release assets and subsequent cask update waits for both platforms' tests, bundles and source archives. No Windows release is claimed as verified before central CI and manual acceptance.

See [Windows builds and releases](../windows-build.md) for the source inventory contract, one-time provisioning, maintainer variables, local build instructions and current external blockers.
