# 01 — ADR-0007: embed libtailscale, superseding ADR-0005

**What to build:** A decision record, following the existing ADR convention in `docs/adr/`, that lets tbis reach a Jellyfin server over a tailnet with nothing installed on the computer. It records:
- **Engine:** Tailscale's official Go `tsnet`, linked in-process as the libtailscale C archive. Chosen over `tailscale-rs` for throughput: gVisor netstack, full NAT traversal and direct connections, MagicDNS names in dial, and an audited codebase. `tailscale-rs` describes itself as unaudited and not for production, and falls back to DERP relays more often.
- **Bridge:** a loopback-only TCP forwarder that sends each local connection over the tailnet to the Jellyfin machine. The address in use points at the forwarder, so the Jellyfin API, images, the Player's mpv and PiP's mpv all go over the tailnet with no proxy settings.
- **Route:** LAN first, with a 1.5 s probe of the server's public info endpoint, then the tailnet if Tailscale is on. The route is chosen once per launch or sign-in.
- **Device:** a persistent node named `tbis-<hostname>`, with its state in the app support directory. Sign-in is a one-time QR code or link to the node's auth URL.
- **Scope:** macOS only.
- **Cost:** a Go toolchain is required at build time (local builds and CI), and the binary grows (size to be filled in by ticket 03).

It marks ADR-0005 (`tailscale-rs` plus an HTTP proxy) as superseded, and lists the options considered: `tailscale-rs`, a bundled `tailscaled` with SOCKS5, and requiring the Tailscale app.

**Blocked by:** None (can start immediately).

**Status:** done (commit 789e2d4)

- [x] ADR-0007 exists and records the engine, bridge, route, device and scope decisions above, each with its reason
- [x] ADR-0005's status is changed to superseded by ADR-0007
- [x] Consequences cover the Go build-time dependency, the loopback port being reachable by other local processes, and a laptop changing networks mid-session needing a restart
