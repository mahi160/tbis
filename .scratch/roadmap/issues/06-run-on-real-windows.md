# 06 — Run tbis on a real Windows PC and record the log

**What to build:** Nothing. The owner reports working Windows playback; record the tested version, hardware, codecs and logs to establish its coverage (`docs/windows.md`).

**Model:** human

**Blocked by:** 02, 03, 04, 05 (they make the log useful).

**Status:** playback reported working by the owner; detailed acceptance evidence pending. App version, GPU/driver, codec coverage and logs were not supplied. Do not infer that all codecs or GPU families pass.

## Steps

1. Download `tbis-*-windows-x86_64.zip` from the newest release, extract it, run `tbis.exe`, sign in.
2. Play one H.264 file, one HEVC file, and one file with AC-3 or DTS audio.
3. For each, write down: did video appear, did audio play, and the exact title and text of any error panel.
4. Copy `%APPDATA%\tbis\tbis.log` and `tbis.previous.log`.
5. Note the Windows version, GPU model and driver version (Task Manager > Performance > GPU), and whether the PC has two GPUs (laptop with integrated and discrete).
6. Paste all of it under **Results** below, and fill the matching rows of the matrix in `docs/windows.md`.

## What the log decides

| Log shows | Next ticket |
| --- | --- |
| `WGL_NV_DX_interop2 is required` or `wglDXOpenDeviceNV failed` | 07 (replace the interop path) |
| Video works, some audio or video codecs fail | 08 (prebuilt libmpv with full codecs) |
| An HTTP or TLS error from mpv | New ticket; attach the `mpv:` lines |
| Everything plays | Mark ADR-0007 accepted and fill the matrix |

## Results

The owner confirmed: “the windows is working!” No version, hardware details, codec list, logs or screenshots accompanied this report.
