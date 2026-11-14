# 26 — Split `jellyfin.rs` into modules

**What to build:** `src/jellyfin.rs` is about 1,630 lines holding four things: data types, the signed-in `Api` client, image URL builders, and sign-in functions. Split it into a `jellyfin/` folder by moving code, with no behaviour change, so each file is one topic.

**Model:** sonnet

**Blocked by:** 21 and 25 (they edit this file; do them first to avoid conflicts).

**Status:** done. `cargo test`: 80 tests before, 80 after. Test-only `fake_server` lives in `mod.rs`'s test module (`pub(super)`), shared by the `auth` and `api` tests; request-only DTOs (`ItemsResult`, `SegmentsResult`, PlaybackInfo types, `device_profile`, image-field constants) moved to `api.rs`.

**Read first:** `src/jellyfin.rs` (all of it), `.scratch/windows/issues/23-split-mpv-module.md` (how the mpv module was split the same way).

## Target layout

| File | Holds |
| --- | --- |
| `src/jellyfin/mod.rs` | `mod` lines, `pub use` re-exports, `normalize_server`, shared request helpers (`send`, `read_json`, `auth_header`) |
| `src/jellyfin/types.rs` | `Session`, `Item`, `Kind`, `Sort`, `UserData`, `PlaybackItem`, stream and track types, `Segment`, `TrickplayInfo`, `PublicUser`, and their `impl` blocks |
| `src/jellyfin/api.rs` | `struct Api` and its request methods (playback, reporting, Library and Home queries, search) |
| `src/jellyfin/images.rs` | The `Api` methods that build image URLs (`poster_url`, `wide_image_url`, `backdrop_url`, `person_image_url`, `trickplay_tile_url`, `image_bytes`) and `blurred` |
| `src/jellyfin/auth.rs` | `public_users`, `login`, the three `quick_connect_*` functions and their private response types |

## Steps

1. Create `src/jellyfin/mod.rs` holding the items the table assigns to it, and delete `src/jellyfin.rs` once everything has moved. Rust accepts only one of the two, so do the move in one sitting.
2. Move items one group at a time, in this order: `types`, `auth`, `images`, `api`. After each group run `cargo check`.
3. Every item other files import must keep its path: re-export from `mod.rs` (`pub use types::*;` and so on) so `crate::jellyfin::Item`, `crate::jellyfin::Api` and `crate::jellyfin::login` still resolve. No file outside `src/jellyfin/` should change.
4. Fields and helpers used across the new files become `pub(super)`; nothing becomes more public than `pub(super)` unless it was already `pub`.
5. Move each `#[cfg(test)] mod` with the code it tests.
6. Change no logic, names, or comments. A moved function's body must be byte-for-byte the same.

## Done when

- [x] `src/jellyfin.rs` is gone and the five files exist
- [x] `git diff --stat` shows changes only under `src/jellyfin/` plus the deleted file
- [x] The test count printed by `cargo test` is the same as before the change (note both numbers in the Status)
- [x] No file is over about 700 lines

**Check:** the gate in `PLAN.md`.
