# 03 — CI runs the tests on Windows; libmpv pin that doesn't expire

**What to build:** `.github/workflows/windows.yml` also runs `cargo test --release` (with `libmpv-2.dll` on `PATH`) and `cargo clippy -- -D warnings`. shinchiro deletes nightly releases after about 30 days, so the pinned `mpv-dev-x86_64` archive gets copied to a release asset in this repo (or a dedicated `libmpv-windows` release) and the workflow downloads it from there, sha256-checked.

**Blocked by:** None — can start immediately. Creating the mirror release needs the user's go-ahead.

**Status:** done

- [ ] `cargo test` and `clippy` pass in the Windows job
- [ ] The workflow no longer downloads from shinchiro's releases
- [ ] Bumping libmpv is one documented edit (tag, file name, sha256)
