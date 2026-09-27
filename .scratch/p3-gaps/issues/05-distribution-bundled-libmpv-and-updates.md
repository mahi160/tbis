# 05 — Distribution: bundled libmpv and auto-update

**What to build:** Revisit ADR-0002 for distributing tbis to other people: the app bundle ships its own libmpv (and a standalone mpv for PiP, ADR-0003), is signed, and can update itself from releases. Starts with a new ADR superseding or amending ADR-0002, including the GPL licensing consequences.

**Blocked by:** None — can start immediately.

**Status:** in progress (bundling done; signing and auto-update remain)

- [x] ADR decides bundle-vs-Homebrew and records licensing obligations (ADR-0004: bundle, GPL-3.0-or-later)
- [ ] If bundling: the app runs on a Mac with no Homebrew mpv installed, including PiP. The bundle is self-contained (the build fails on any leftover Homebrew reference, and the bundled mpv loads all 47 libraries from `Contents/Frameworks`), but it hasn't been run on a Mac without Homebrew yet.
- [ ] Developer ID signing and notarization (needs an Apple Developer account)
- [x] Release artifacts include the GPL Corresponding Source for bundled components (scripts/gpl-sources.sh, attached by the release workflow)
- [x] Update check finds a newer release and installs it with user confirmation (built-in GitHub updater in src/update.rs: checks on launch and from the user menu, verifies the zip's GitHub SHA-256 and the app's signature, swaps the bundle, relaunches; the swap and relaunch haven't been run end to end yet)
