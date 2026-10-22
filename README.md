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

## Performance

### Compared with other Jellyfin clients

This benchmark, run in October 2026, compares tbis 0.4.0 with Jellyfin Media Player 1.12.0, Moonfin 2.7.0, and jellyfin-mpv-shim 3.2.0 (with mpv 0.41.0). Jellyfin Desktop 2.0 was installed but not measured.

**Setup.** MacBook Air M4 (Mac16,12, 4 performance and 6 efficiency cores, 8-core GPU, 16 GB) on macOS 27.2, plugged in, Low Power Mode off, built-in 2560×1664 display. The server ran Jellyfin 12.2.0 on the local network. Every client played *Men in Black* (1997): a 41.2 GiB, 67 Mbps MKV with 4K HEVC Main 10 Dolby Vision video (including the enhancement layer), TrueHD Atmos 7.1 as the default audio track, and 42 PGS subtitle tracks.

**Method.** Each client was measured once a second for 3 minutes of playback, starting at 10:00 after a 15-second warm-up, and for 60 seconds idle on the home screen. The measurement covers the client's whole process tree, so mpv-shim's numbers include its Python GUI and its mpv process. CPU, memory, and Energy Impact come from `top`. GPU % and power are for the whole Mac and come from `ioreg` and `powermetrics`. Incoming network traffic comes from `nettop`. Startup is the time from launch until the first window is on screen.

Only tbis played the original file directly. The server converted it for every other client, each with its default settings, to H.264 video and AAC audio at 14.36 Mbps. To compare like with like, tbis was also measured with a bitrate cap, which made the server send it the same 14.36 Mbps H.264 stream.

Playback of the same 14.36 Mbps H.264 stream:

| Metric | tbis | Jellyfin Media Player | jellyfin-mpv-shim | Moonfin |
|---|---|---|---|---|
| CPU average / 95th percentile | 24.2% / 27.7% | 34.9% / 41.6% | 24.9% / 26.9% | 11.3% / 25.6% |
| Energy Impact average | 25.6 | 36.8 | 25.7 | 13.0 |
| Memory average / peak | 857 / 1016 MB | 800 / 905 MB | 1005 / 1173 MB | 943 / 1205 MB |
| GPU busy (whole Mac) | 30.9% | 27.5% | 21.7% | 8.5% |
| GPU power (whole Mac) | 290 mW | 267 mW | 284 mW | 31 mW |
| CPU + GPU power (whole Mac) | 1559 mW | 1580 mW | 2073 mW | 1362 mW |

Playback of the original 4K Dolby Vision file, which only tbis played directly: 31.7% CPU on average, 33.0 Energy Impact, 1402 MB memory, 59.6% GPU busy (588 mW), 1918 mW for the CPU and GPU together, and 62 Mbps received.

Idle on the home screen, and startup:

| Metric | tbis | Jellyfin Media Player | jellyfin-mpv-shim | Moonfin |
|---|---|---|---|---|
| Startup to first window | 94 ms | 851 ms | no window | 671 ms |
| CPU average | 2.3% | 1.7% | 0.3% | 31.7% |
| Energy Impact average | 3.4 | 2.8 | 1.4 | 38.2 |
| Memory average | 258 MB | 461 MB | 416 MB | 775 MB |
| GPU busy (whole Mac) | 14.0% | 12.5% | 5.8% | 83.2% |

**Verdicts.**

- **tbis** was the only client that played the original 4K Dolby Vision file without help from the server. Doing so had a lower Energy Impact (33.0) than Jellyfin Media Player had on the converted 14 Mbps stream (36.8), although the whole Mac drew more power (1918 mW against 1580 mW), mostly in the GPU. On the same converted stream, tbis used about 30% less CPU and Energy Impact than Jellyfin Media Player. It also started the fastest and used the least memory when idle.
- **Jellyfin Media Player** idles efficiently. It can't direct play this file, and on the converted stream it used the most CPU of the clients.
- **jellyfin-mpv-shim** is cheap on its own, but you control it from a browser or another Jellyfin client. Its high whole-Mac power during playback includes that browser, which its own numbers leave out. It also can't direct play this file.
- **Moonfin** shows the lowest playback numbers. Its video is probably drawn by a system video layer whose work is counted to macOS's window server, not to Moonfin, so those numbers are likely understated (unverified). When idle it keeps the GPU at about 83% busy, which is by far the most of any client. It can't direct play this file either.

**Limits.** Each configuration ran once, so differences of a few percent are noise. The server's conversion work is not included in any of the numbers. GPU and power figures cover the whole Mac, so background activity adds to them.

### Compared with Photon

This compares tbis with Photon during playback of the same movie, fullscreen, on macOS. Each app was sampled once for 60 seconds with `top -l 31 -s 2`, and only the main process was measured.

| Metric | tbis | Photon |
|---|---|---|
| CPU average | 10.7% | 37.1% |
| CPU peak | 28.7% | 101.7% |
| Energy Impact average | 10.7 | 37.3 |
| Memory footprint | 1385 MB | 409 MB |
| Resident memory (RSS) | 247 MB | 277 MB |
| Threads | 27 | 44 |

These numbers are rough. Energy Impact is an estimate from macOS, not a measurement in watts. The run did not confirm that both apps used the same codec, resolution, or hardware decoding. tbis had been running for a while before the test, so its memory footprint may include cached images.

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
