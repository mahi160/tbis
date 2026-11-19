# 08 — Windows libmpv from a prebuilt package

**What to build:** Replace the old Windows source build with a pinned, checksummed prebuilt libmpv development package, keeping the `MPV_LIB_DIR` contract that `build.rs` reads. This implementation has landed; native CI and the codec acceptance checklist remain to be recorded.

**Model:** opus

**Blocked by:** Owner decision 1 in `PLAN.md` (this changes how the GPL source offer is met). Do ticket 06 first when possible: its log shows whether missing codecs are a real problem.

**Status:** code landed; owner reports Windows playback working, tested version unspecified. The latest standalone Windows run (`42976947593`) failed in the old source-build step, so it is not evidence for this implementation. No mirror runs were listed. Current Windows CI, release packaging and codec-matrix acceptance remain unverified. Owner chose shinchiro's package with a link-only source offer (`components/sources.txt`), as jellyfin-mpv-shim does. Pinned `mpv-dev-x86_64-20261111-git-bcde6b93cc.7z` (mpv v0.42.0-38): inspected on macOS, its only non-system import is `vulkan-1.dll` (hard import), so setup builds the Khronos loader from pinned source (`vulkan-sdk-1.4.357.0`, static CRT) and the zip ships it. The DLL is 116 MB (98 MB of code, not debug info), so the zip grows. Lua is enabled; `Handle::init` already handles `osc`. mpv's SMTC is compiled in but off by default under libmpv. Updater needs no change: `*.dll` and `components/` are already managed.

**Read first:** `docs/windows-build.md`, `docs/adr/0004-bundle-libmpv-and-license-as-gpl.md`, `scripts/setup-windows-deps.ps1`, `scripts/windows-common.ps1`, `scripts/bundle-windows.ps1`, `.github/workflows/windows.yml`, `.github/workflows/mirror-windows-deps.yml`, `.scratch/cross-platform/PLAN.md` (the "Reusable now" list describes this approach).

## Steps

1. Pick the package and pin it by URL and SHA-256 in `scripts/windows-deps.json`. It must contain `libmpv-2.dll` and the headers, be built for x86_64, and publish the source revisions it was built from.
2. In `setup-windows-deps.ps1`: download, verify the checksum, extract, generate `mpv.lib` from the DLL's exports with `dumpbin` and `lib /def`, and set `MPV_LIB_DIR`.
3. Keep the import gate that checks which system DLLs `libmpv-2.dll` needs, and ship any extra DLLs the package requires beside `tbis.exe` in `bundle-windows.ps1`.
4. Replace the source archive with a small text file listing the package URL, its checksum and the upstream source revisions, and state in `README.md` and ADR-0004 how the source offer is met.
5. Delete the source-build recipe, its manifest entries, its patches and its tests once the new path passes CI.
6. Confirm `osc` handling in `Handle::init` still works: a package built with Lua has the `osc` option, and the code already handles both cases.

## Done when

- [ ] The Windows workflow passes without compiling mpv or FFmpeg
- [ ] A release has no multi-gigabyte asset
- [ ] `tbis.exe --self-test` passes in CI
- [ ] A file with AC-3, E-AC-3, DTS, VP9 and AV1 each plays on a Windows PC (human check)
- [ ] ADR-0004 and `docs/windows-build.md` describe the new source offer

**Check:** the Windows workflow, then a human run of the acceptance list above.
