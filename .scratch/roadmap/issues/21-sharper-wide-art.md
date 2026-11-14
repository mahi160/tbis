# 21 — Sharper wide art on Retina

**What to build:** Wide cards are 288 points across (`design::size::wide_w`, 72 units of 4 pt) but their image is requested 480 px wide, less than the 576 px a 2× display draws. Request 600 px.

**Model:** haiku

**Blocked by:** None.

**Status:** ready-for-agent

**Read first:** `src/jellyfin.rs` (`fn wide_image_url`), `src/design.rs` (`wide_w` in `mod size`).

## Steps

1. In `fn wide_image_url`, change `fillWidth=480` to `fillWidth=600`.
2. Above the `format!`, add the comment `// wide_w is 288 pt; 600 px covers a 2x display`.
3. Update any test that asserts the old URL (`rg -n "fillWidth=480" src`).

## Done when

- [ ] Wide images are requested at 600 px
- [ ] No `fillWidth=480` remains

**Check:** the gate in `PLAN.md`.
