# 16: Gate Picture-in-Picture once, through the platform layer

**What to build:** Picture-in-Picture code compiles on every OS again. A single platform capability (for example a `SUPPORTS_PIP` constant next to each OS's video surface) decides whether the PiP button, the `P` key binding and the PiP row in the shortcuts sheet appear. Windows shows none of them; macOS and Linux behave exactly as before. The ~15 scattered `cfg(not(target_os = "windows"))` gates in design tokens, video components, mpv, the Player, its controls, shortcuts and the noop window module are removed, and the Windows video surface's `move_to` stops being dead code kept 'for the contract'.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] PiP module, action, design tokens, `pip_frame` and `Mpv::move_to` compile on every OS without target cfg gates
- [ ] One capability check controls the PiP button, key binding and shortcut row
- [ ] Windows: no PiP button, `P` does nothing, shortcut sheet has no PiP row
- [ ] macOS/Linux: PiP works as before
- [ ] noop window functions (`hide_title_buttons`, `lock_aspect`) are no longer cfg-gated
- [ ] `cargo check` passes on macOS; Windows CI build passes

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
