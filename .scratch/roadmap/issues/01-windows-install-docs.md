# 01 — Windows install docs name the right download

**What to build:** The README still says no Windows release exists and sends people to CI artifacts. Releases now carry four assets, and the biggest one (`tbis-<version>-windows-sources.tar`, about 1.4 GB) is source code, not the app. Rewrite the Windows install section so a user downloads the right file.

**Model:** haiku

**Blocked by:** None.

**Status:** done

**Read first:** `README.md` (the `### Windows 10/11 (x86_64)` section), `docs/windows.md` (the `## Logs` section).

## Steps

1. In `README.md`, replace the paragraph under `### Windows 10/11 (x86_64)` with numbered steps:
   1. Download `tbis-*-windows-x86_64.zip` from Releases (link to `https://github.com/mahi160/tbis/releases`).
   2. Extract the whole zip into a folder you can write to, and run `tbis.exe`.
2. Under the steps, add one sentence: the `*-sources.tar` files on the release page are source code for the GPL licence and are not needed to run the app.
3. Add one sentence: playback on Windows is still being verified; when a video fails, the log is at `%APPDATA%\tbis\tbis.log`.
4. Keep the existing link to `docs/windows-build.md`.

## Done when

- [x] The Windows section names `tbis-*-windows-x86_64.zip` and never tells the user to open CI artifacts
- [x] It says what the `sources.tar` files are
- [x] It names the log path

**Check:** `rg -n "windows-x86_64.zip|sources.tar|tbis.log" README.md` prints all three.
