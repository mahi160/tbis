# Unpublished Linux Flatpak candidate

This is a Wayland test candidate, not a Linux release. X11 video attachment is not implemented, so the manifest does not grant X11 access. The candidate has not been built or tested on Linux. The existing release workflows do not publish it.

## Build or download

Dispatch **Flatpak candidate** in GitHub Actions on the branch containing the packaging files. The resulting `tbis-flatpak-candidate-x86_64` artifact contains the bundle, its SHA-256, and the source revision and installed SDK/runtime commits used for the build. The workflow does not publish a GitHub release.

To build locally on Ubuntu 24.04 x86-64 with Git and Rust installed, run these commands from the repository root:

```sh
sudo apt-get update
sudo apt-get install -y flatpak flatpak-builder
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --user -y flathub \
    org.freedesktop.Platform/x86_64/26.08 \
    org.freedesktop.Sdk/x86_64/26.08 \
    org.freedesktop.Sdk.Extension.rust-stable/x86_64/26.08
sh scripts/build-flatpak-candidate.sh HEAD candidate
```

The script archives only the specified commit, so uncommitted edits and developer-local files are not included. Cargo sources are vendored from the lockfile before the sandbox build; Cargo runs offline inside it. The manifest pins libmpv, libass, and libplacebo source commits and uses the Freedesktop runtime's FFmpeg. It builds libmpv without the standalone mpv CLI. The Flatpak runtime and SDK are pinned to the 26.08 series; its updates can change the installed commits, which are recorded in `provenance.txt`.

For release acceptance, the candidate must come from the prepared release revision, not from a moving branch. Use that revision as the script's first argument. Before public distribution, verify the license notices and corresponding source offer for all shipped dependencies, and obtain human approval to attach the exact tested bundle by digest. Do not rebuild it during promotion.

## Install and test

On a clean Linux desktop without host mpv, open a terminal from a native Wayland session. From the directory containing the unzipped artifact:

```sh
sha256sum -c tbis-linux-x86_64.flatpak.sha256
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --user ./tbis-linux-x86_64.flatpak
flatpak run --user --branch=candidate io.github.mahi160.tbis
```

Repeat the [native Wayland checklist](linux-wayland-validation.md) inside the candidate. For the trace launch use `flatpak run --user --branch=candidate --env=WAYLAND_DEBUG=client io.github.mahi160.tbis` instead of the native binary. The Flatpak already isolates per-app data and does not expose an X11 socket. Do not copy native test credentials or settings into the sandbox. Note the bundle digest and record these additional results:

- Sign in, browse the library, and stream a title. Change a preference, quit the app, and confirm that the preference persists after restarting.
- Check that the desktop does not idle while playing and resumes normal idle behavior after playback stops. Record compositor-specific limitations.
- Where PiP is offered, enter it, return to the main window, then close and reopen playback. Record any lost video, unresponsive controls, or stranded windows.

Permissions to verify: the Wayland socket creates the window; PulseAudio provides sound; DRI provides GPU rendering and hardware decoding where available; network access reaches Jellyfin; `org.freedesktop.ScreenSaver` is the only explicit session-bus permission and matches the Linux idle-inhibition call. No host filesystem access or broad device permission is granted.

The native and sandboxed Wayland results must be recorded separately. This candidate cannot satisfy the X11 or XWayland rows in the [Linux support plan](linux-support-plan.md).
