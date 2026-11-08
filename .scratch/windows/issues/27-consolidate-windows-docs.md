# 27: Consolidate the Windows docs

**What to build:** The Windows validation story (build steps, toolchain, export counts) is stated once in the Windows build doc; ADRs record decisions only. Script counts and descriptions match the finished scripts and CI.

**Blocked by:** 24: Generate Windows dependency facts from the manifest; 25: Generate Windows build inputs instead of hand-building them; 26: Deduplicate the Windows CI workflows

**Status:** ready-for-agent

- [ ] Validation narrative appears in one doc
- [ ] ADR 0004 and the Windows video-surface ADR contain decisions, not run logs
- [ ] Script counts and tooling descriptions are consistent and current
- [ ] The README's Windows section is short and points at the Windows build doc instead of repeating CI run details

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
