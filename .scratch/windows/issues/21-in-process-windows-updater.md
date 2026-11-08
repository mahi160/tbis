# 21: Make the Windows updater one in-process Rust flow

**What to build:** The Windows updater stages, validates, unpacks and swaps the update entirely inside the app, with no PowerShell helper. It reads the downloaded zip from memory (the `zip` crate is approved), rejects unsafe entries before writing anything, unpacks to a staging folder beside the install, renames the running `tbis.exe` and bundled DLLs aside (Windows allows renaming, not overwriting, files that are in use), moves the new files in, and relaunches. The next launch deletes the old files. A failed swap moves the old files back so the install keeps working. Start with a short spike confirming the rename-aside approach works for the running exe and loaded DLLs in this bundle layout; if it does not, fall back to keeping only a minimal post-exit Apply helper and record why.

The PowerShell updater and its tests, the Inspect/Extract/Apply modes, the JSON plan/entries/state files, the ready/cancel file handshake, the subprocess timeouts, the raw zip attribute bitmasks, and the `#[path]` include that runs Windows update tests on other OSes are removed. The rule for safe Windows file names exists once in Rust, in a normal cross-platform module so its tests run on every OS. Validation stays proportionate: the download is already SHA-256 checked, so keep path-traversal, link/special-file and allowlist checks, and drop defences that only guard against a malicious release.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Spike outcome recorded in this ticket (rename-aside works or not)
- [ ] `zip` added as a dependency; no PowerShell updater remains (or only a minimal Apply helper if the spike failed, with the reason recorded)
- [ ] Unsafe paths, links/special files, unexpected names and case-colliding entries are rejected before anything is written (tests cover the existing rejection cases)
- [ ] A failed swap leaves the old install working (test covers it)
- [ ] Old files are cleaned up on the next launch
- [ ] No `#[path]` test include in the platform module
- [ ] An end-to-end Windows update (stage, swap, relaunch) works in CI or a manual run

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
