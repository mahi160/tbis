# 40 — The build follows libmpv after `brew upgrade mpv`

**What to build:** `build.rs` asks pkg-config where libmpv is, and Homebrew answers with a versioned folder (`/opt/homebrew/Cellar/mpv/0.41.0_12/lib`). Cargo caches that answer. After `brew upgrade mpv` the folder is gone and linking fails with `ld: library 'mpv' not found` until someone runs `cargo clean -p tbis`. Make cargo rerun the build script when that folder changes or disappears.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). The non-Windows build tracks every pkg-config libmpv link directory. The Windows branch is unchanged. Both current build-script output files contain `cargo:rerun-if-changed=/opt/homebrew/Cellar/mpv/0.42.0/lib`; verbose cargo build was fresh and did not replay that output. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `build.rs` (the last two lines of `fn main`).

## Steps

1. In `build.rs`, replace the last statement of `fn main`:

   ```rust
   pkg_config::probe_library("mpv").expect("libmpv not found via pkg-config (brew install mpv)");
   ```

   with:

   ```rust
   let mpv = pkg_config::probe_library("mpv")
       .expect("libmpv not found via pkg-config (brew install mpv)");
   // brew keeps libmpv in a versioned folder; rerun when an upgrade moves it
   for path in &mpv.link_paths {
       println!("cargo:rerun-if-changed={}", path.display());
   }
   ```

   `link_paths` is a public `Vec<PathBuf>` field of `pkg_config::Library`. Cargo reruns a build script whenever a `rerun-if-changed` path is missing.

Do not touch the Windows branch above it.

## Done when

- [x] `cargo build` succeeds
- [ ] `cargo build -vv 2>&1 | rg "rerun-if-changed=.*mpv"` (after `touch build.rs`) prints the libmpv folder

**Check:** the gate in `PLAN.md`.
