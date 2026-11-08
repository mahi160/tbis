# 22: Tidy the cross-platform update module

**What to build:** Each OS declares its release asset suffix as a constant instead of a fallible function; Linux's update stubs shrink to only what is reachable (bundle path is `None`, so check/install/relaunch never run). The doc comments the branch deleted from the update module (module overview, `check`, `install`, `newer`, `bundle_path`, `relaunch`) are restored and reflect the multi-OS design.

**Blocked by:** 21: Make the Windows updater one in-process Rust flow

**Status:** ready-for-agent

- [ ] No `asset_suffix() -> Result`; a per-OS constant is used
- [ ] Linux update module contains no unreachable error paths beyond what compilation requires
- [ ] Update module docs restored
- [ ] Update tests pass; macOS and Windows update flows unchanged
- [ ] The release checksum is validated once, where the release is parsed, not again on install and verify
- [ ] Linux's do-nothing update pieces live in the shared noop module, per the platform convention

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
