# 03 — mpv logs at `info`

**What to build:** mpv's messages are requested at `warn`, which drops the lines that explain a failed start: which decoder opened, whether hardware decoding was used, which audio output was picked. Request `info` instead. Repeated lines are already collapsed by `LogDedup` in `src/mpv/events.rs`.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified)

**Read first:** `src/mpv/mod.rs` (`fn init`), `src/mpv/events.rs` (`LogDedup` and where log messages are handled).

## Steps

1. In `src/mpv/mod.rs`, `fn init`, change `mpv_request_log_messages(self.0, c"warn".as_ptr());` to `c"info".as_ptr()`.
2. Add a one-line comment above it: `// info: decoder, hwdec and audio-output choices, needed to diagnose a failed start`.
3. Run the app (`cargo run --release`), play any video for ten seconds, and look at the terminal output. Confirm new `mpv:` lines appear and none of them contains the server address or token (the log redacts lines with `://`; see `src/log.rs`, `fn redact`).

## Done when

- [x] Log level is `info`
- [ ] A playback run prints decoder and audio-output lines
- [ ] No token or server URL appears in the output

**Check:** the gate in `PLAN.md`, plus step 3. When you cannot run the app, set Status to `done (runtime unverified)`.
