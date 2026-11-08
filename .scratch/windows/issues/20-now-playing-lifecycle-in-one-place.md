# 20: Create and tear down Now Playing in one place

**What to build:** The Player's Now Playing sync receives the window and both creates and drops the system media session (SMTC on Windows, MPNowPlaying on macOS), instead of creation living inline in render and teardown in the sync function. The unused `App` parameter goes away.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Session creation and teardown live in one function
- [ ] Render no longer constructs `NowPlaying` inline
- [ ] Media keys, title, artwork and seek work on Windows and macOS as before
- [ ] `cargo check` passes on macOS; Windows CI build passes
- [ ] SMTC session creation reads top to bottom: one fallible create function, errors mapped once at the call site (no closure inside a closure)

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
