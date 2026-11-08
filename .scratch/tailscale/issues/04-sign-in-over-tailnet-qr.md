# 04 — Sign in to Jellyfin over the tailnet with a QR code

**What to build:** On the login page, a "Use Tailscale" toggle that reveals a "Tailnet address" field (machine name or 100.x IP, with port). When the toggle is on and the user continues:
- If this install's node isn't authorized yet, the page shows the auth URL as a QR code and as a clickable link, and waits until the node is running. Scanning with a phone signed in to Tailscale and approving the device is enough; nothing is installed on the computer.
- tbis starts the loopback forwarder to the tailnet address and points the server in use at it.
- Loading the user list, Quick Connect and password sign-in all go over the tailnet.

The toggle and the tailnet address are saved to config, so the next launch remembers them. In this ticket, Tailscale on always means the tailnet route; LAN-first comes in ticket 05.

New UI pieces (the QR code view, the toggle row) are components built on the design tokens. Buttons go through the shared button component.

**Blocked by:** 02 (server in use), 03 (libtailscale wrapper).

**Status:** done (runtime unverified: the hotspot QR demo and second sign-in haven't been run). Shipped together with ticket 05's LAN-first route, so Tailscale on means "LAN first, tailnet fallback" rather than always the tailnet. The address parser, forwarder loopback binding and config round-trip are unit tested.

- [ ] Demo: on a network that can't reach the server's LAN address (for example, a phone hotspot), a fresh install signs in through the QR flow and lands on Home
- [ ] A second sign-in on the same install skips the QR code (the node state persists)
- [ ] The forwarder listens on loopback only
- [ ] The tailnet address parser accepts a machine name or a 100.x IP, with or without a port, and rejects anything else (unit tested)
- [ ] The Tailscale settings round-trip through config, and old configs without them still load (unit tested)
