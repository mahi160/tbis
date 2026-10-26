---
status: accepted
---

# Reach a Jellyfin server over a tailnet by embedding libtailscale

This supersedes ADR-0005. Some users run Jellyfin behind Tailscale instead of exposing it publicly. tbis reaches such a server with nothing installed on the computer: no Tailscale app, no `tailscaled`, no root.

- **Engine.** tbis links Tailscale's official Go `tsnet` in-process, built as the libtailscale C archive (`go build -buildmode=c-archive`). It is chosen over `tailscale-rs` for throughput and maturity: it uses the gVisor netstack, does full NAT traversal so peers get direct connections instead of staying on DERP relays, resolves MagicDNS names when dialing, and is an audited codebase. `tailscale-rs` describes itself as unaudited and not for production, and falls back to DERP relays more often, which caps streaming bitrate.
- **Bridge.** A TCP forwarder listens on loopback only (`127.0.0.1:<port>`) and sends each local connection over the tailnet to the Jellyfin machine. The server address in use points at the forwarder, so the Jellyfin API (`reqwest`), images, and the Player's in-process mpv (ADR-0001), which PiP also uses (ADR-0006), all go over the tailnet with no proxy settings. A plain byte forwarder is used instead of ADR-0005's HTTP proxy because it needs no HTTP handling and no per-client proxy configuration.
- **Route.** LAN first: tbis probes the saved server's public system info endpoint (`/System/Info/Public`) with a 1.5 s timeout. If it answers, everything runs over the LAN at full speed and the tailnet node is not started. Otherwise, if Tailscale is on in Settings, tbis starts the node and the forwarder and uses the tailnet. The route is chosen once per launch or sign-in, so requests never switch routes mid-session.
- **Device.** tbis runs one persistent node named `tbis-<hostname>`, with its state in the app support directory (`support_dir.rs`), so it appears as one stable device in the tailnet admin console. Signing in is a one-time QR code or clickable link to the node's auth URL; the persisted state keeps it authorized after that.
- **Scope.** macOS only, matching tbis's current platform support.

## Considered Options

- **`tailscale-rs` with a local HTTP proxy (ADR-0005).** Keeps the build Cargo-only, but it is pre-1.0, unaudited and not meant for production, lacks MagicDNS, and falls back to DERP relays more often, which is too slow for high-bitrate video. Rejected.
- **Spawn a bundled `tailscaled`** in userspace-networking mode with `--socks5-server`. Mature, but a second exception to ADR-0001's "nothing spawned" rule, another bundled binary with its own login and state-file lifecycle, and every client (including mpv) would need SOCKS5 settings. Rejected.
- **Require the user to install the Tailscale app.** The status quo this removes; it defeats the point for the non-technical users tbis targets (ADR-0004). Rejected.

## Consequences

- A Go toolchain is required at build time, for local builds and CI, to produce the libtailscale archive alongside the existing pkg-config/mpv linking.
- The binary grows by the size of the Go runtime and `tsnet`: the `dist` binary went from 16.0 MB (15,990,720 bytes) to 36.2 MB (36,208,304 bytes), +20.2 MB. It was measured with a temporary reference to `tailscale::Node` from the app, because the linker only pulls in the Go archive once app code calls it. Until sign-in over the tailnet lands, only the test binary links it. `tsnet`'s `ts_omit_*` build tags for unused features (SSH, Taildrop, AWS, Kubernetes and so on) shrank the archive by only about 6%, so they are not used.
- Go 1.27 builds for macOS 13 or later, so the whole app now targets macOS 13 (`.cargo/config.toml` and `LSMinimumSystemVersion` in `scripts/bundle.sh`), up from macOS 11.
- libtailscale is a git submodule (`vendor/libtailscale`), compiled by `build.rs` with `go build -buildmode=c-archive`. The bundle carries the licenses of libtailscale, Go and every Go module linked in, under `licenses/libtailscale`.
- The forwarder's loopback port can be reached by any other process on the same computer, which can then talk to the Jellyfin server through tbis's tailnet node. It only forwards to the one Jellyfin machine, and Jellyfin still requires its own authentication.
- Because the route is chosen once, a laptop that changes networks mid-session (for example leaving home) keeps its old route until tbis is restarted or the user signs in again.
- libtailscale is BSD-3-Clause, compatible with tbis's GPL-3.0-or-later.
