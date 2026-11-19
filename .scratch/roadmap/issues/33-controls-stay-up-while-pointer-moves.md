# 33 — Controls stay up while the pointer moves

**What to build:** The Player's controls hide 3 seconds after they appear, even while the pointer is still moving over them, then pop back on the next move. Moving the pointer must restart the 3 second wait.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). Pointer movement restarts the hide timer without requesting a redraw while controls are already visible. Paused, menu-open, scrubbing and error guards are unchanged. `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` succeeded (86 tests); Cargo reports a dependency future-incompatibility warning for `block v0.1.6`. Windows CI not run.

**Read first:** `src/player/controls.rs` (`fn show_controls`, and the `.on_mouse_move(...)` call in `impl Render for PlayerView`).

## Why it happens

`on_mouse_move` calls `show_controls` only when the controls are hidden, so the hide timer started by the first move is never restarted.

## Steps

1. In `src/player/controls.rs`, split the timer out of `show_controls`. Replace the whole `show_controls` function with these two; the body of the spawned task is unchanged:

   ```rust
   pub(super) fn show_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
       if !self.controls_visible {
           crate::platform::cursor::show();
       }
       self.controls_visible = true;
       self.arm_hide(window, cx);
       cx.notify();
   }

   /// Restarts the wait before the controls hide.
   fn arm_hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
       self._hide = cx.spawn_in(window, async move |this, cx| {
           cx.background_executor().timer(HIDE_CONTROLS_AFTER).await;
           this.update(cx, |this, cx| {
               let busy = this.paused || this.menu_open || this.playback.scrubbing.is_some();
               if !busy && this.playback.error.is_none() {
                   this.controls_visible = false;
                   // the timeline goes with the controls, and no leave event follows
                   this.playback.hover_time = None;
                   hide_cursor();
                   cx.notify();
               }
           })
           .ok();
       });
   }
   ```

2. In `impl Render for PlayerView`, replace the `.on_mouse_move(...)` call with:

   ```rust
   .on_mouse_move(cx.listener(|this, _, window, cx| {
       if this.controls_visible {
           // nothing on screen changes, so no redraw: only the wait restarts
           this.arm_hide(window, cx);
       } else {
           this.show_controls(window, cx);
       }
   }))
   ```

Do not call `show_controls` on every move: it calls `cx.notify()`, which would redraw the window for every pointer event.

## Done when

- [ ] Moving the pointer continuously over the video keeps the controls up for as long as it moves
- [ ] The controls hide 3 seconds after the pointer stops
- [ ] Paused, an open menu, and scrubbing still keep the controls up

**Check:** the gate in `PLAN.md`. Runtime: play a video, move the pointer in circles for 10 seconds, and watch that the controls never blink.
