# 24: Generate Windows dependency facts from the manifest

**What to build:** There is one allowlist of system DLLs the bundle may import, using a prefix rule for API sets (`api-ms-win-*`, `ext-ms-*`), shared by the Python source tooling and the PowerShell import check. Component names, source directory mappings and the expected subproject set come from the Windows dependency manifest rather than hard-coded special cases in the Python tooling.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Exactly one system-DLL allowlist exists
- [ ] No hard-coded component set, directory special cases or expected-subproject set in the Python tooling
- [ ] Python tooling tests pass
- [ ] Source archive and libmpv build in CI produce the same results as before

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
