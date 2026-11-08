# 05 — LAN-first route at launch, with tailnet fallback

**What to build:** When tbis launches with a saved session, or the user signs in, it picks the route once:
- It probes the saved server address (public system info, 1.5 s timeout). If that answers, everything runs over the LAN at full speed.
- If not, and Tailscale is on, it starts the node and the forwarder and points the server in use at the tailnet.
- If neither works, the existing error or expired-session flow shows.

Home, Search, Movie detail, Series detail, the Player (including Autoplay and trickplay) and PiP all work on whichever route was picked.

**Blocked by:** 04.

**Status:** ready-for-agent

- [ ] At home with Tailscale on, the LAN route is used and the tailnet node is not started
- [ ] Away from home with Tailscale on, the app restores the session over the tailnet and plays a Movie and an Episode, including in PiP
- [ ] With Tailscale off, behaviour matches today
- [ ] Route selection is unit tested with a fake probe: LAN ok, LAN timeout with Tailscale on, LAN timeout with Tailscale off, and both failing
