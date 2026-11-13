# 22: Tidy the cross-platform update module

**What to build:** Each OS declares its release asset suffix as a constant instead of a fallible function; Linux's update stubs shrink to only what is reachable (bundle path is `None`, so check/install/relaunch never run). The doc comments the branch deleted from the update module (module overview, `check`, `install`, `newer`, `bundle_path`, `relaunch`) are restored and reflect the multi-OS design.

**Blocked by:** 21: Make the Windows updater one in-process Rust flow

**Status:** implemented; macOS and Windows clippy and macOS tests pass. Linux is unverified (can't compile here), read only.

- [x] No `asset_suffix() -> Result`; a per-OS constant is used
- [x] Linux update module contains no unreachable error paths beyond what compilation requires (`install`/`relaunch` are `unreachable!`, required to compile; `ASSET_SUFFIX` is `""`)
- [x] Update module docs restored
- [x] Update tests pass; macOS and Windows update flows unchanged
- [x] The release checksum is validated once, where the release is parsed, not again on install and verify (`Update.sha256` is a parsed `[u8; 32]`)
- [x] Linux's do-nothing update pieces live in the shared noop module, per the platform convention

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
