# 10 — Exclusive audio only with passthrough

**What to build:** `AudioStyle::mpv_options` always sets `audio-exclusive`, writing `no` when passthrough is off. That overrides a user's own `audio-exclusive=yes` in `mpv.conf`. Set the option only when passthrough is on.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified)

**Read first:** `src/config.rs` (`AudioStyle::mpv_options` and its tests), `src/player/mod.rs` (where `config.audio.mpv_options()` is chained), `src/player/pip.rs` (any other caller: `rg -n "mpv_options" src`).

## Steps

1. Change `AudioStyle::mpv_options` to return `Vec<(&'static str, String)>`.
2. Always push `audio-spdif` and `af` as today. Push `("audio-exclusive", "yes".to_string())` only when `self.passthrough` is true.
3. Fix the callers the compiler reports; chaining a `Vec` into the existing iterator works unchanged.
4. Update or add a test in `src/config.rs`: with passthrough off, no entry is named `audio-exclusive`; with it on, the entry is `yes`.

## Done when

- [x] `audio-exclusive` is absent when passthrough is off
- [x] Test covers both cases

**Check:** the gate in `PLAN.md`.
