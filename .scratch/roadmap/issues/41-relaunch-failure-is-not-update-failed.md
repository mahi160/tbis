# 41 — A failed restart is not reported as a failed update

**What to build:** Installing an update replaces the app on disk and then restarts it. When only the restart fails, the app says "Update failed", although the new version is already installed. Say what actually happened.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). Installation and relaunch results are handled separately; restart failures are logged and say the update is installed. The final code lives in src/app/updates.rs after ticket 49. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/app.rs` (`fn install_update`), `src/update.rs` (`pub async fn install`, `pub fn relaunch`).

## Steps

1. In `fn install_update` in `src/app.rs`, replace the body of the `cx.spawn_in(window, async move |this, cx| { ... })` block with:

   ```rust
   let installed = cx
       .background_spawn(async move { update::install(http, &update).await })
       .await;
   // the inner result is the restart: by then the new version is already on disk
   let result = installed.map(|()| update::relaunch());
   this.update_in(cx, |_, window, cx| match result {
       Ok(Ok(())) => cx.quit(),
       Ok(Err(err)) => {
           crate::log::write(&format!("relaunch after update failed: {err}"));
           let note =
               Notification::info("The update is installed. Quit and reopen tbis to finish.");
           window.push_notification(note, cx);
       }
       Err(err) => {
           let note = Notification::error(format!("Update failed: {err}"));
           window.push_notification(note, cx);
       }
   })
   .ok();
   ```

## Done when

- [ ] A download or install error still shows "Update failed: ..."
- [ ] A restart error shows "The update is installed. Quit and reopen tbis to finish." and is written to the log

**Check:** the gate in `PLAN.md`.
