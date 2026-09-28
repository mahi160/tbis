# 07 — Jellyseerr / Overseerr request integration

**What to build:** From a Movie or Series detail page, a user can submit a request to a configured Jellyseerr/Overseerr instance for content not already in the library. Server URL and API key are set once in Settings (`docs/audit-2026-09.html` §3, §8).

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Jellyseerr/Overseerr server URL and API key can be configured in Settings
- [ ] Detail pages for items not in the library show a "Request" action
- [ ] Submitting a request confirms success or surfaces the server's error
- [ ] No Jellyseerr/Overseerr configured: the Request action is hidden, no broken UI
