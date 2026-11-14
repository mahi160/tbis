# 24 — Log dropped mpv errors in the Player

**What to build:** The Player discards the result of about forty mpv calls with `let _ = mpv.something(...)`. A failed seek, pause or track switch then leaves no trace. Route them through one helper that logs the failure.

**Model:** haiku

**Blocked by:** None.

**Status:** done

**Read first:** `src/player/mod.rs`, `src/player/controls.rs`, `src/player/playback.rs`, `src/player/pip.rs`, `src/player/controls/chrome.rs` (every `let _ =`: `rg -n "let _ = " src/player`), `src/log.rs` (`write`).

## Steps

1. In `src/player/mod.rs`, add:

   ```rust
   /// Logs a failed mpv call the Player can carry on without.
   pub(super) fn logged<T, E: std::fmt::Display>(what: &str, result: Result<T, E>) {
       if let Err(err) = result {
           crate::log::write(&format!("mpv: {what} failed: {err}"));
       }
   }
   ```

2. For each `let _ = mpv.<method>(...)` under `src/player/`, replace it with `logged("<method>", mpv.<method>(...));` using the method name as the label, for example `logged("set_pause", mpv.set_pause(!self.paused));`. Use `super::logged` or `crate::player::logged` as the module requires.
3. Leave a `let _ =` in place when the value is not a `Result` (the compiler tells you: `logged` will not accept it) or when it is not an mpv call, such as a channel send.
4. Run `rg -n "let _ = mpv|let _ = self.mpv" src/player`; it must print nothing.

## Done when

- [x] No discarded mpv `Result` remains under `src/player/`
- [x] Behaviour is unchanged when calls succeed

**Check:** the gate in `PLAN.md`.
