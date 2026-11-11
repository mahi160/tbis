# 14 — In-app updates on Windows

**What to build:** `src/update.rs` only knows `tbis.app`; on Windows the manual check says "Updates only work in the installed tbis.app." Find the install from the exe's folder, download the release's Windows zip, check it, and swap `tbis.exe` and `libmpv-2.dll` (a running exe can be renamed but not overwritten), then relaunch. User text mentions tbis, not tbis.app.

**Blocked by:** 13.

**Status:** needs-human (code landed; re-verify end to end after 21 rewrites the swap)

- [ ] An older build offers, installs and relaunches into the newer one
- [ ] A failed download or swap leaves the old install working
- [ ] No macOS-only wording on Windows
