# 07 — mpv path lists use `;` on Windows

**What to build:** `ShaderProfile::mpv_value` (`src/shaders.rs`) joins shader paths with `:`. mpv's manual: path list options use `:` on Unix and `;` on Windows, and Windows paths contain `C:` anyway, so every shader profile breaks on Windows. Use the platform separator. Check the other paths handed to mpv (screenshots, `support_dir`) for the same problem.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] `mpv_value` uses `;` on Windows and `:` elsewhere, with a test
- [ ] Shader profiles apply on Windows (verified with ticket 06)
- [ ] Screenshots land in the Pictures folder on Windows
