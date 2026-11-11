# 12 — Title bar and window chrome on Windows

**What to build:** The title bar is laid out for macOS traffic lights (`design::window::TRAFFIC_LIGHTS` and the brand lead offset in `design.rs`). On Windows, gpui-kit's `TitleBar` draws its own minimize, maximize and close buttons on the right: check the brand's left padding, the centred content, dragging, double-click to maximize, Aero snap, and the window border. Keyboard hints in the UI must say Ctrl, not ⌘. `platform::windows::window::prepare` stays a no-op unless a fix needs it.

**Blocked by:** 01.

**Status:** needs-human (code landed; screenshots pending)

- [ ] Brand, centre content and caption buttons sit correctly at 100% and 150% scale
- [ ] Drag, double-click maximize and snap work
- [ ] No ⌘ appears anywhere in the UI on Windows
