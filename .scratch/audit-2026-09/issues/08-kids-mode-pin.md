# 08 — Kids Mode / parental PIN

**What to build:** A PIN-gated restricted browsing mode. When enabled, the app requires a PIN to leave a restricted library view (or to enter the full library), keeping unsuitable content out of reach without a separate account (`docs/audit-2026-09.html` §3, §8).

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] A PIN can be set and changed from Settings
- [ ] Kids Mode restricts the browsed library (e.g. to items marked suitable) until the correct PIN is entered
- [ ] Wrong PIN is rejected without revealing anything about the restricted content
- [ ] Kids Mode state (on/off) persists across launches
