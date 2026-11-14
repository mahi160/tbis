# 22 — Debounce mpv.conf saves

**What to build:** Settings writes `mpv.conf` to disk on every keystroke in the mpv.conf box. Write it once the user pauses typing for half a second.

**Model:** haiku

**Blocked by:** None.

**Status:** done, runtime unverified. Quitting the app (cmd-Q) within the half second may not drop `SettingsView`, so that last edit may not be written.

**Read first:** `src/settings.rs` (`SettingsView::new`, the `_save_mpv_conf` subscription), `src/login.rs` (`fn load_users` shows the debounce pattern: a stored `Task` replaced on each change, starting with a timer).

## Steps

1. Add a field `_write_mpv_conf: Task<()>` to `SettingsView`, initial `Task::ready(())`.
2. Change the subscription closure so that on `InputEvent::Change` it replaces the task:

   ```rust
   let text = state.read(cx).value().to_string();
   this._write_mpv_conf = cx.spawn(async move |_, cx| {
       cx.background_executor()
           .timer(std::time::Duration::from_millis(500))
           .await;
       if let Err(err) = mpv::save_user_config(&text) {
           crate::log::write(&format!("saving mpv.conf failed: {err}"));
       }
   });
   ```

   The closure's first parameter is `_` today; rename it to `this`.
3. The last edit must survive Settings closing within the half second, because dropping the view cancels the task. In the same subscription, also store the latest text in a new field `mpv_conf_pending: Option<String>`. Inside the task, after the save, clear it with `this.update(cx, |this, _| this.mpv_conf_pending = None).ok();` (name the task closure's first parameter `this` for that). Add `impl Drop for SettingsView` that calls `mpv::save_user_config` on `mpv_conf_pending` when it is `Some`, logging a failure the same way.

## Done when

- [x] Typing a line writes the file once, half a second after the last key
- [x] Typing and leaving Settings at once still saves the text
- [x] A save failure is still logged

**Check:** the gate in `PLAN.md`. Runtime: type in the box, leave Settings immediately, reopen it, and the text is there.
