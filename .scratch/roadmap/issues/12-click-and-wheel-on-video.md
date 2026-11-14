# 12 — Click to pause, double-click fullscreen, wheel volume

**What to build:** The video area ignores the mouse except for showing the controls. Make a single click toggle pause, a double click toggle fullscreen, and the scroll wheel change volume.

**Model:** haiku

**Blocked by:** None.

**Status:** ready-for-agent

**Read first:** `src/player/controls.rs` (`fn render` from `let chrome =` to the end, `fn toggle_pause`, `fn toggle_fullscreen`, `fn volume_up`, `fn volume_down`), `src/player/mod.rs` (`VOLUME_STEP`). In gpui: `ClickEvent::click_count` in `~/.cargo/registry/src/*/gpui-pre-0.3.9/src/interactive.rs` and `on_scroll_wheel` in `src/elements/div.rs`.

## Steps

1. The actions take an action struct as their second argument. Split the bodies so the mouse can call them: add `fn pause_toggle(&mut self, window, cx)`, `fn fullscreen_toggle(&mut self, window, cx)` and `fn nudge_volume(&mut self, up: bool, window, cx)` holding the existing bodies, and make `toggle_pause`, `toggle_fullscreen`, `volume_up` and `volume_down` call them.
2. In `fn render`, build a click layer that fills the Player and sits under every overlay:

   ```rust
   let click_layer = div()
       .id("video-click")
       .absolute()
       .inset_0()
       .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
           if event.click_count() >= 2 {
               // the first click of the pair paused; undo it, then go fullscreen
               this.pause_toggle(window, cx);
               this.fullscreen_toggle(window, cx);
           } else {
               this.pause_toggle(window, cx);
           }
       }))
       .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
           let up = event.delta.pixel_delta(px(1.)).y > px(0.);
           this.nudge_volume(up, window, cx);
       }));
   ```

   Read `ScrollWheelEvent` in gpui first and use its real field and method names. `px(0.)` and `px(1.)` are the allowed zero and one literals.
3. Add it as the first child of the root (before `.children(error)`), so the error panel, banners and chrome stay above it and keep their own clicks.
4. Skip the layer when `self.playback.error.is_some()` or `self.pip.is_some()`.

## Done when

- [ ] One click on the video toggles pause
- [ ] A double click toggles fullscreen and leaves the pause state as it was
- [ ] Scrolling over the video changes volume and shows the volume indicator
- [ ] Clicks on the control bar, menus, skip button and up-next card do what they did before

**Check:** the gate in `PLAN.md`, then try each of the four points in a running Player.
