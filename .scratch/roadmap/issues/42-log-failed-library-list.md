# 42 — Log a failed Library list

**What to build:** With the Libraries setting on Separate, Movies and Series load the list of Libraries for their tabs. When that request fails, the tabs disappear and nothing is recorded. Keep the fallback (the grid still shows All) and write the error to the log.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). Library-list failures are logged while preserving the All-grid fallback. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/library.rs` (`fn refresh`).

## Steps

1. In `fn refresh` in `src/library.rs`, replace:

   ```rust
   let mut libraries = api.libraries().await.unwrap_or_default();
   ```

   with:

   ```rust
   let mut libraries = api.libraries().await.unwrap_or_else(|err| {
       crate::log::write(&format!("libraries lookup failed: {err}"));
       Vec::new()
   });
   ```

   Leave the comment above it (`// the grid can still show All without the tabs`) in place.

## Done when

- [ ] The grid still loads when the Library list fails
- [ ] The failure appears in the log

**Check:** the gate in `PLAN.md`.
