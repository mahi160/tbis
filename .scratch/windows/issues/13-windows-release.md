# 13 — Windows release in the prod pipeline

**What to build:** `release.yml` gets a Windows job: build with the `dist` profile, package `tbis-$VERSION-windows-x86_64.zip` (`tbis.exe`, `libmpv-2.dll`, licence files), and attach it to the same GitHub release as the macOS zip. The GPL sources archive covers the bundled libmpv/FFmpeg (ADR-0004). The exe carries the app icon and version info. README documents how to build on Windows (`MPV_LIB_DIR`).

**Blocked by:** 03, 06.

**Status:** done

- [ ] A prod push publishes the Windows zip next to the macOS one
- [ ] The exe shows the tbis icon in Explorer and the taskbar
- [ ] The GPL obligations in ADR-0004 are met for the Windows zip
