# 27 — Signed releases

**What to build:** The updater checks a download against a SHA-256 that comes from the same GitHub release (`src/update.rs`, `fn verify_checksum`), which catches a corrupt download but not a replaced release. On macOS it runs `codesign --verify`, which passes for any self-consistent signature (`src/platform/macos/update.rs`). Sign releases with a key only the owner holds, and have the updater verify with the public half built into the app.

**Model:** human for the keys, then sonnet for the code

**Blocked by:** The owner creating and storing the keys.

**Status:** needs-human

**Read first:** `src/update.rs`, `src/update/bundle.rs`, `src/platform/macos/update.rs`, `.github/workflows/release.yml`, `.scratch/p3-gaps/issues/05-distribution-bundled-libmpv-and-updates.md`.

## Human steps

1. Decide between an Apple Developer ID (also removes the "can't check for malicious software" prompt on macOS) and a self-managed signing key (free, covers both platforms, does not remove the macOS prompt). They can be combined.
2. For a self-managed key: generate an Ed25519 key pair, store the private key as a GitHub Actions secret, and put the public key in this ticket.

## Code steps (after the keys exist)

1. Release workflow: sign each app zip and upload the signature file as a release asset beside it.
2. Updater: download the signature, verify the zip against the embedded public key before extraction, and refuse the update with a clear message when it fails. Keep the SHA-256 check as the corruption check.
3. macOS with a Developer ID: add the Team ID requirement to the `codesign` verification.
4. Tests: a valid signature passes, a changed byte fails, a missing signature fails.

## Done when

- [ ] A release carries a signature per app zip
- [ ] The updater refuses a zip whose signature does not match the embedded key
- [ ] The three tests pass
