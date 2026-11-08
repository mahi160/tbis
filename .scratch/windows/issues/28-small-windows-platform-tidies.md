# 28: Small Windows platform tidies

**What to build:** Small cleanups once PiP gating lands: the Windows video surface no longer shows the cursor when it drops; the Windows power request is tracked as a single handle; the macOS traffic-light position and brand lead come from one per-OS optional constant instead of paired cfgs; showing the controls doesn't call cursor-show on every mouse move.

**Blocked by:** 16: Gate Picture-in-Picture once, through the platform layer

**Status:** ready-for-agent

- [ ] Video surface drop has no cursor side effect
- [ ] Power request holds one handle and clears cleanly
- [ ] Title-bar geometry uses a single per-OS constant
- [ ] Cursor still reappears on mouse move and hides after the controls timeout on every OS
- [ ] Shader path lists use the standard library's platform path-list join instead of a hand-written separator switch
- [ ] Windows startup (log, self-test flag, runtime init) is one platform init call in `main`

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
