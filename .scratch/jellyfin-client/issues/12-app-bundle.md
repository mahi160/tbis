# 12 — App bundle

**What to build:** `scripts/bundle.sh` builds a release binary and writes `target/tbis.app` with an Info.plist, the binary, and a placeholder app icon, so tbis can be launched from Finder or the Dock. libmpv stays an external Homebrew dependency (ADR-0002).

**Blocked by:** None

**Status:** done (superseded by ADR-0004: the bundle now also ships libmpv)

- [x] `scripts/bundle.sh` produces `target/tbis.app`
- [x] The app launches from Finder, shows its icon in the Dock, and plays video
- [x] Launched from Finder, the app still raises its open-file limit and finds mpv for PiP
