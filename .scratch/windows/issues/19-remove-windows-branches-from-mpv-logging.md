# 19: Remove Windows branches from mpv event logging

**What to build:** mpv log lines, decoder changes and the end-of-session summary go through one logging path on every OS, with redaction applied in one place. The mpv event thread no longer takes Windows-only parameters, the `hwdec-current` name mapping exists once (or is removed if decoder names are not sensitive), and secret extraction from the auth header no longer lives inline in `Mpv::new`.

**Blocked by:** 18: Make the Windows log reliable and simple

**Status:** done (runtime unverified on Windows; Windows CI build not run, only the cross-target clippy)

- [x] The mpv event thread and `Mpv::new` contain no `target_os` cfg blocks for logging
- [x] Decoder mapping is not duplicated
- [x] Secrets from the auth header are redacted from mpv logs on every OS
- [ ] `cargo check` passes on macOS; Windows CI build passes

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
