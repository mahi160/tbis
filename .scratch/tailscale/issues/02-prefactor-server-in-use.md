# 02 — Prefactor: separate the saved server address from the address in use

**What to build:** Today the saved Session's server URL is also the URL every request uses. Split the two: the Session keeps the server address the user entered, and the running app reads a separate, in-memory "server in use" for every Jellyfin API call, image URL, trickplay URL and mpv/PiP load. For now the server in use is always the saved address, so behaviour is unchanged. Later tickets point it at the tailnet forwarder without changing any call sites.

**Blocked by:** None (can start immediately).

**Status:** done. The server in use is `Api`'s `server` field, seeded from the saved address in `app.rs` `main_screen`.

- [x] Every request, image, trickplay, stream and PiP URL is built from the server in use, never directly from the saved Session
- [x] The server in use is never written to config
- [x] No visible behaviour change; existing tests pass
- [x] A unit test shows that URLs follow the server in use when it differs from the saved address
