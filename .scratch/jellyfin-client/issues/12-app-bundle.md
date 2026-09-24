# 12 — App bundle

**What to build:** `scripts/bundle.sh` builds a release binary and writes `target/tbis.app` with an Info.plist, the binary, and a placeholder app icon, so tbis can be launched from Finder or the Dock. libmpv stays an external Homebrew dependency (ADR-0002).

**Blocked by:** None

**Status:** ready-for-agent

- [ ] `scripts/bundle.sh` produces `target/tbis.app`
- [ ] The app launches from Finder, shows its icon in the Dock, and plays video
- [ ] Launched from Finder, the app still raises its open-file limit and finds mpv for PiP
