# 26: Deduplicate the Windows CI workflows

**What to build:** One composite action covers checking out the prepared release revision and setting up Python and MSVC; the Windows, dependency-mirror and release workflows use it instead of copying the steps.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Prepared-revision checkout exists once
- [ ] Python/MSVC setup exists once
- [ ] Windows, mirror and release workflows green

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
