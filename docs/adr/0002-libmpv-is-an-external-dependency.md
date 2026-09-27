---
status: superseded by ADR-0004
---

# libmpv is an external dependency, not bundled

The app links the user's installed libmpv dynamically (`brew install mpv` on macOS, found through pkg-config at build time), and we never ship libmpv or its ffmpeg dylibs inside our binary or `.app`. Because our artifacts contain no GPL code, both the source and the binaries stay permissively licensed. That avoids the licensing work Photon ran into (see Photon ADR-0004/0011). The cost is that the app does not launch at all without mpv installed, and macOS fails with a dyld error rather than a friendly screen. We accepted that over the extra complexity of loading libmpv at runtime with `dlopen`.
