# 36 — Player error carries its own title

**What to build:** The error panel picks its title by checking how the error text starts (`error.starts_with("Cannot start player")`). Rewording a message silently changes the title. Store the title next to the message instead. Nothing the user sees changes.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). PlayerError stores a fixed title and detail; startup, load and playback failures choose their title explicitly. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/player/playback.rs` (`struct Playback`, `fn fail`, both `self.fail(...)` calls), `src/player/mod.rs` (`PlayerView::new` around `Mpv::new`, `fn start`), `src/player/controls.rs` (`let error = ...` in `impl Render for PlayerView`).

## Steps

1. In `src/player/playback.rs`, above `struct Playback`, add:

   ```rust
   /// What the error panel shows: a fixed title over the cause.
   #[derive(Clone)]
   pub(super) struct PlayerError {
       pub(super) title: &'static str,
       pub(super) detail: SharedString,
   }

   pub(super) const START_FAILED: &str = "The player could not start";
   pub(super) const LOAD_FAILED: &str = "This item could not be loaded";
   pub(super) const STOPPED: &str = "Playback stopped";
   ```

2. In `struct Playback`, change `pub(super) error: Option<SharedString>,` to `pub(super) error: Option<PlayerError>,`.

3. Replace `fn fail` with:

   ```rust
   /// Shows `message` under `title` in the error panel and records it in the log.
   pub(super) fn fail(&mut self, title: &'static str, message: String) {
       crate::log::write(&format!("player error: {message}"));
       self.playback.error = Some(PlayerError {
           title,
           detail: message.into(),
       });
   }
   ```

4. In `src/player/playback.rs`, change both `self.fail(format!("Playback failed: {err}"))` calls to `self.fail(STOPPED, format!("Playback failed: {err}"))`.

5. In `src/player/mod.rs`:
   - change `use playback::Playback;` to `use playback::{LOAD_FAILED, Playback, PlayerError, START_FAILED, STOPPED};`
   - in `PlayerView::new`, in the `Err(err)` arm of `match Mpv::new(...)`, change `(None, Some(message.into()))` to:

     ```rust
     (
         None,
         Some(PlayerError {
             title: START_FAILED,
             detail: message.into(),
         }),
     )
     ```

   - in `fn start`, change `self.fail(format!("Cannot load item: {err}"))` to `self.fail(LOAD_FAILED, format!("Cannot load item: {err}"))`, and `Err(err) => self.fail(err),` to `Err(err) => self.fail(STOPPED, err),`.

6. In `src/player/controls.rs`, in `impl Render for PlayerView`, inside `let error = self.playback.error.clone().map(|error| { ... })`:
   - delete the whole `let title = if error.starts_with(...) { ... } else { ... };` statement
   - change `error_panel(title, error, actions, Tone::Video, cx)` to `error_panel(error.title, error.detail, actions, Tone::Video, cx)`.

Every other use of `self.playback.error` is `.is_none()` or `.is_some()` and needs no change.

## Done when

- [x] No `starts_with("Cannot` remains: `rg -n 'starts_with\("Cannot' src` prints nothing
- [x] The three titles and all message texts are unchanged

**Check:** the gate in `PLAN.md`.
