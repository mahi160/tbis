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
