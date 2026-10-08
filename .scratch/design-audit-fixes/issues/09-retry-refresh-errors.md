# 09 — Retry on background refresh errors

**What to build:** When a background refresh fails on Home or Library but older content is still on screen, the error line comes with a small Retry button. Retry runs the same refresh again.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [x] Home and Library show Retry next to a refresh error while older content is still on screen
- [ ] Retry shows its loading state while the refresh runs, and the error clears on success
- [x] The full-screen error state (no content at all) works as before
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
