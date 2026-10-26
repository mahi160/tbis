# 03 — Spike: libtailscale builds and links into tbis

**What to build:** Bring libtailscale into the build, pinned as a git submodule. The build compiles it as a Go C archive and links it statically, with the macOS frameworks it needs. Add a thin, safe Rust wrapper over the C API: start a node with a state directory and hostname, read the status (backend state and auth URL, from the status JSON), dial a TCP address on the tailnet, and log out (close the node and delete its state). Install Go in CI and in the release bundling script.

This is the riskiest step, so it comes first: it proves the cgo link works and measures how much bigger the shipped binary gets.

**Blocked by:** None (can start immediately).

**Status:** done (the CI release run hasn't been tried, and the full `scripts/bundle.sh` run was blocked locally by an existing Homebrew brotli `@rpath` error; the license step was run on its own). Go 1.27 targets macOS 13, so the app's minimum went from 11 to 13. Dist binary 16.0 MB → 36.2 MB. The wrapper is `src/tailscale.rs`. `delete_state` signs out with no node running (ticket 06).

- [x] `cargo build` builds and links libtailscale when Go is installed; a missing Go toolchain fails with a clear message
- [x] CI and the release bundle both build with libtailscale
- [x] A test starts a node in a temp directory and reads a needs-login state with a non-empty auth URL (ignored or gated if CI has no network)
- [x] The dist-profile binary size before and after is recorded in ADR-0007 (ticket 01)
- [x] libtailscale's BSD-3-Clause license is covered in the bundled license notices
