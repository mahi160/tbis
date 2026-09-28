# 04 — Quick Connect login

**What to build:** The login screen offers "Sign in with Quick Connect" alongside username/password. The user requests a code from the server, tbis displays it, polls Jellyfin's Quick Connect endpoints until the user approves it from another device, then completes session setup the same way `login()` does today (`docs/audit-2026-09.html` §3, §8).

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [x] "Sign in with Quick Connect" button next to Sign in, on the same panel
- [x] `/QuickConnect/Initiate` requested; the returned code is shown, then `/QuickConnect/Connect` is polled every 2s until `Authenticated`
- [x] Approval calls `/Users/AuthenticateWithQuickConnect` and builds the same `Session` shape as `login()`, going through the same `LoggedIn` event
- [x] A 401 from Initiate (Quick Connect disabled) surfaces as "Quick Connect is turned off on this server."
- [x] A 404 from polling (expired/invalid code) or Cancel both drop `quick_connect` state and return to the normal form; cancelling replaces the polling task, same cancel-by-replace pattern as `load_users`
