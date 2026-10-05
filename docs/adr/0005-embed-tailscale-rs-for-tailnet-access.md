---
status: proposed
---

# Reach a Jellyfin server over a tailnet using the `tailscale` crate, not the Tailscale app

Some users run Jellyfin behind Tailscale instead of exposing it publicly. Today that means installing the separate Tailscale app before tbis can reach the server. Instead, tbis links the official `tailscale` crate (`tailscale/tailscale-rs`, BSD-3-Clause) directly. At login, an "Connect to Tailscale" option starts a `Device` with no auth key, polls `Device::is_authorized()` for the resulting login URL, and shows it as a QR code and a clickable link. Once the user finishes the browser login, the device is authorized and its keys are persisted to a file in the app's data directory, so this only happens once.

Because `tailscale-rs` runs an entirely userspace network stack (no TUN interface, no root, nothing installed system-wide), only code that calls `Device::tcp_connect` directly can reach the tailnet. tbis therefore runs a small local HTTP proxy (`127.0.0.1:<port>`) inside the app that bridges each request to `Device::tcp_connect`, and points both the `reqwest` client (Jellyfin API) and libmpv's `--http-proxy` option (ADR-0001: mpv does its own networking in-process) at it.

## Considered Options

- **Require the user to install the Tailscale app.** Rejected as the status quo we're trying to remove; it also defeats the point for non-technical users tbis targets (ADR-0004).
- **`tsnet` crate wrapping `libtailscale`** (Tailscale's official Go `tsnet`, built as a C archive via `go build -buildmode=c-archive`). More mature and battle-tested than `tailscale-rs`, but requires adding a Go toolchain to the build and CI pipeline and linking a cgo static library, on top of the pkg-config/mpv linking tbis already does. Rejected for now to keep the build Cargo-only; revisit if `tailscale-rs`'s gaps (see Consequences) become blocking.
- **Spawn a bundled `tailscaled` binary** in userspace-networking mode with `--socks5-server`, the pattern Tailscale documents for exactly this kind of embedding. More mature than `tailscale-rs` and gives a SOCKS5 proxy for free, but is a second exception to ADR-0001's "nothing spawned" rule (after PiP's standalone mpv) and adds another bundled binary and its login/state-file lifecycle to manage. Rejected in favor of a pure-Rust, in-process dependency.

## Consequences

- `tailscale-rs` is pre-1.0 and explicitly early/WIP: no compatibility guarantees yet, and it doesn't support exit nodes, subnet routers, MagicDNS, or private DERP relays. None of that is needed to reach one Jellyfin peer by tailnet IP or name (`Device::peer_by_name`), but a tailnet setup that depends on those features won't work through tbis yet.
- Adds a local proxy component tbis has to build and maintain: an HTTP proxy service that bridges to `Device::tcp_connect`, used by both `reqwest` and mpv. This is new surface area, not just calling the crate.
- Covers macOS only for now, matching tbis's current platform support; `tailscale-rs` also supports Linux and Windows if tbis grows there.
- `tailscale-rs`'s BSD-3-Clause license is compatible with tbis's GPL-3.0-or-later.
