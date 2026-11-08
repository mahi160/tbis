# 16: Gate Picture-in-Picture once, through the platform layer

**What to build:** Picture-in-Picture code compiles on every OS again. A single platform capability (for example a `SUPPORTS_PIP` constant next to each OS's video surface) decides whether the PiP button, the `P` key binding and the PiP row in the shortcuts sheet appear. Windows shows none of them; macOS and Linux behave exactly as before. The ~15 scattered `cfg(not(target_os = "windows"))` gates in design tokens, video components, mpv, the Player, its controls, shortcuts and the noop window module are removed, and the Windows video surface's `move_to` stops being dead code kept 'for the contract'.

**Blocked by:** None (can start immediately)

**Status:** implemented; clippy and tests pass on macOS, clippy passes for the Windows target; runtime behaviour unverified on Windows and Linux, Windows CI not yet run

- [x] PiP module, action, design tokens, `pip_frame` and `Mpv::move_to` compile on every OS without target cfg gates
- [x] One capability check (`platform::video::SUPPORTS_PIP`) controls the PiP button and key binding; the shortcut row follows the key binding
- [x] Windows: no PiP button, `P` does nothing, shortcut sheet has no PiP row (by construction; not run)
- [x] macOS/Linux: PiP works as before (code unchanged; not run on Linux)
- [x] noop window functions (`hide_title_buttons`, `lock_aspect`) are no longer cfg-gated
- [ ] `cargo check` passes on macOS; Windows CI build passes (macOS and Windows-target clippy pass; CI not run)

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
