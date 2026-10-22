# 09 — Release build tuning

**What to build:** The release build uses default settings. Enable thin LTO and a single codegen unit, then record the effect: release build time, app binary size, and a rough startup/scroll check.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] The release profile enables thin LTO and one codegen unit
- [ ] Before/after release build time and binary size are recorded in the commit message
- [ ] The release workflow still builds, bundles and signs the app
