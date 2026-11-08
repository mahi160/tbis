# 06 — Settings: Tailscale status, route in use, sign out of Tailscale

**What to build:** A Tailscale row in Settings showing:
- the node's status: connected, needs login, or off
- the route this launch is using: LAN or tailnet

It also has a "Sign out of Tailscale" action that removes this install's node state, so the next tailnet connection asks for the QR code again. Signing out of Tailscale doesn't sign the user out of Jellyfin.

**Blocked by:** 05.

**Status:** ready-for-agent

- [ ] The row shows the correct status and route on the LAN and away from home
- [ ] Sign out of Tailscale removes the node state, and the next tailnet connection shows the QR code
- [ ] The row is built from the existing settings components and design tokens
