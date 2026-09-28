# 04 — Quick Connect login

**What to build:** The login screen offers "Sign in with Quick Connect" alongside username/password. The user requests a code from the server, tbis displays it, polls Jellyfin's Quick Connect endpoints until the user approves it from another device, then completes session setup the same way `login()` does today (`docs/audit-2026-09.html` §3, §8).

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Login screen has a Quick Connect option next to username/password
- [ ] A code is requested and shown to the user, with polling until approved or the code expires
- [ ] Approval completes login and session setup identically to the existing username/password path
- [ ] Server with Quick Connect disabled: the option doesn't appear, or fails with a clear message
- [ ] Expired/cancelled code returns the user to the login screen without a stuck spinner
