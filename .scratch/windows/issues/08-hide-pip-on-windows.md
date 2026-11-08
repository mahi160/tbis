# 08 — Hide Picture-in-Picture on Windows

**What to build:** PiP is out of scope on Windows. The PiP button and the `TogglePip` shortcut are not shown or bound on Windows, so users never reach `Surface::move_to`'s error.

**Blocked by:** None — can start immediately.

**Status:** superseded by 16 (PiP gated once through the platform layer)

- [ ] No PiP button or shortcut on Windows
- [ ] The shortcuts overlay doesn't list PiP on Windows
- [ ] macOS PiP is unchanged
