# 13 — Format the branch

**What to build:** The branch passes `cargo fmt --check` again. Tickets 01–12 left formatting drift in a few files. Formatting only; no code changes.

**Blocked by:** None — can start immediately

**Status:** done (visual check pending)

- [x] `cargo fmt --check` passes
- [x] The diff is whitespace and line-wrapping only
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
