# tbis

A native macOS client for [Jellyfin](https://jellyfin.org), built with [GPUI](https://gpui.rs) and playing video through an embedded [mpv](https://mpv.io).

## Install

With [Homebrew](https://brew.sh), which also handles the Gatekeeper step below:

```sh
brew install --cask mahi160/tbis/tbis
```

Or by hand:

1. Download the latest `tbis-*-macos-arm64.zip` from [Releases](https://github.com/mahi160/tbis/releases). It needs an Apple Silicon Mac running macOS 11 or later.
2. Unzip it and move `tbis.app` to your Applications folder.
3. Open it once. macOS will say it can't check the app for malicious software, because tbis isn't signed with an Apple Developer ID yet. To allow it, do one of the following:
   - Open **System Settings → Privacy & Security**, scroll down to the message about tbis, and click **Open Anyway**.
   - Or run this in Terminal:

     ```sh
     xattr -dr com.apple.quarantine /Applications/tbis.app
     ```

You only need to do this once. The app bundles everything it needs, including mpv, so you don't need Homebrew.

## Updates

tbis checks for a newer release each time it starts, and you can check yourself with **Check for Updates…** in the user menu. When you accept an update, tbis downloads it, checks it against the checksum GitHub records for the release, replaces the app, and restarts. Updates installed this way don't need the Gatekeeper step again. If tbis is in a folder you can't write to, the update fails and you can download it from Releases instead.

## Build from source

You need Rust and Homebrew's mpv (`brew install mpv`).

```sh
cargo run --release         # run straight from the build
sh scripts/bundle.sh        # build a self-contained target/tbis.app
```

## Releases

Pushing to the `prod` branch builds and publishes a release through GitHub Actions (`.github/workflows/release.yml`). The version is bumped from the commit messages since the last release:

- a breaking change (`feat!:` or `BREAKING CHANGE`) bumps the major version
- any `feat:` commit bumps the minor version
- anything else bumps the patch version

The first release has no earlier tag to bump from, so it uses the version in `Cargo.toml` as is.

## License

tbis is licensed under the [GNU General Public License v3.0 or later](LICENSE). The app bundles GPL builds of mpv and FFmpeg (see [ADR-0004](docs/adr/0004-bundle-libmpv-and-license-as-gpl.md)). Each release includes a `sources` archive with the source code of tbis and of every bundled library, and the app carries their license files in `tbis.app/Contents/Resources/licenses`.
