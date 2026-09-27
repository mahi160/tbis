# 05 — Distribution: bundled libmpv and auto-update

**What to build:** Revisit ADR-0002 for distributing tbis to other people: the app bundle ships its own libmpv (and a standalone mpv for PiP, ADR-0003), is signed, and can update itself from releases. Starts with a new ADR superseding or amending ADR-0002, including the GPL licensing consequences.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] ADR decides bundle-vs-Homebrew and records licensing obligations
- [ ] If bundling: the app runs on a Mac with no Homebrew mpv installed, including PiP
- [ ] Update check finds a newer release and installs it with user confirmation
