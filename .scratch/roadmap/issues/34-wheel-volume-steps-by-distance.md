# 34 — Wheel volume steps by distance, not by event

**What to build:** Scrolling over the video changes the volume by 5 for every scroll event. A trackpad swipe sends dozens of events, so the volume jumps straight to 0 or 100. Step once per 40 px of trackpad travel; a mouse wheel notch stays one step.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). Trackpad pixel deltas accumulate in 40 px steps with reversal reset; wheel notches keep one volume step. Two regression tests added. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/player/controls.rs` (the `.on_scroll_wheel(...)` call in `impl Render for PlayerView`, `fn nudge_volume`), `src/player/mod.rs` (`struct PlayerView`, the `Self { ... }` literal in `PlayerView::new`).

## Steps

1. In `src/player/mod.rs`, add a field to `struct PlayerView` after `volume_hud`:

   ```rust
   /// Trackpad scroll not yet turned into a volume step, in px; see `controls::wheel_step`.
   wheel: f32,
   ```

   and `wheel: 0.,` after `volume_hud: false,` in the `Self { ... }` literal in `PlayerView::new`.

2. In `src/player/controls.rs`, below the `SPEEDS` constant, add:

   ```rust
   /// Trackpad pixels scrolled per volume step.
   const WHEEL_PX_PER_STEP: f32 = 40.;

   /// Adds a trackpad scroll of `dy` px to `carry`. Returns the direction to step
   /// (1 up, -1 down) once a full step has built up, else 0; a reversal starts over.
   fn wheel_step(carry: &mut f32, dy: f32) -> i32 {
       if dy * *carry < 0. {
           *carry = 0.;
       }
       *carry += dy;
       if carry.abs() < WHEEL_PX_PER_STEP {
           return 0;
       }
       let direction = if *carry > 0. { 1 } else { -1 };
       *carry = 0.;
       direction
   }
   ```

3. Replace the `.on_scroll_wheel(...)` call on the click layer with:

   ```rust
   .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
       let direction = match event.delta {
           // one wheel notch is one step
           ScrollDelta::Lines(lines) if lines.y > 0. => 1,
           ScrollDelta::Lines(lines) if lines.y < 0. => -1,
           ScrollDelta::Lines(_) => 0,
           ScrollDelta::Pixels(pixels) => wheel_step(&mut this.wheel, pixels.y.as_f32()),
       };
       if direction != 0 {
           this.nudge_volume(direction > 0, window, cx);
       }
   }))
   ```

   `ScrollDelta` is gpui's enum (`Pixels(Point<Pixels>)`, `Lines(Point<f32>)`) and should already be in scope through `use gpui_kit::*;`. If the compiler cannot find it, stop and write that under Status.

4. At the end of `src/player/controls.rs`, add:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::wheel_step;

       #[test]
       fn a_trackpad_swipe_steps_once_per_forty_pixels() {
           let mut carry = 0.;
           // 20 events of 4 px: 80 px
           let steps: i32 = (0..20).map(|_| wheel_step(&mut carry, 4.)).sum();
           assert_eq!(steps, 2);
       }

       #[test]
       fn reversing_direction_starts_over() {
           let mut carry = 0.;
           assert_eq!(wheel_step(&mut carry, 39.), 0);
           assert_eq!(wheel_step(&mut carry, -39.), 0);
           assert_eq!(wheel_step(&mut carry, -1.), -1);
       }
   }
   ```

## Done when

- [ ] A two-finger swipe over the video changes the volume by a few steps, not to 0 or 100
- [ ] One mouse wheel notch changes the volume by 5
- [x] Both new tests pass

**Check:** the gate in `PLAN.md`.
