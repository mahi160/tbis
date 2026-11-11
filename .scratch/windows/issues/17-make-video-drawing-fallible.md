# 17: Make video drawing fallible on every OS

**What to build:** Every OS's video renderer reports draw failures through its `draw` result, and the mpv render loop handles a failure uniformly: it logs it, ends playback with an error the Player shows, and stops rendering. The Windows-only `take_error` side channel, the cloned event sender passed into the render thread, and the Windows-only frame counters disappear from the shared render loop (counters are either kept for all OSes or dropped).

**Blocked by:** None (can start immediately)

**Status:** done (runtime unverified on Windows; Linux not compiled)

- [x] `Renderer::draw` returns a result on macOS, Linux and Windows
- [x] The shared render loop contains no `target_os` cfg blocks
- [x] A Windows render failure still ends playback with the 'close and reopen the player' error
- [x] macOS/Linux playback unchanged
- [x] `cargo check` passes on macOS; Windows target clippy passes (Windows CI build not run)

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
