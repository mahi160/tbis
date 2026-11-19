# 45 — Sharper posters on Retina

**What to build:** Posters are requested 320 px wide. A poster card is 176 points wide (`poster_w = 44` spacing units of 4 px), which is 352 device pixels on a Retina display, and grid cards stretch wider than that. Posters are therefore upscaled and slightly soft. Request them 400 px wide. Ticket 21 did the same for wide art.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). poster_url requests fillWidth=400. Cache bounds and the blurred URL test input are unchanged. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/jellyfin/images.rs` (`pub fn poster_url`).

## Cost to know about

Each decoded poster grows from about 0.6 MB to about 1 MB. Home and Search keep at most 120 images (`IMAGES_KEPT`), so the worst case grows by about 40 MB. The owner accepts this by scheduling the ticket; do not change `IMAGES_KEPT`.

## Steps

1. In `pub fn poster_url` in `src/jellyfin/images.rs`, change `fillWidth=320` to `fillWidth=400`. Change nothing else in the URL.

Do not edit the test near the bottom of the file that contains `fillWidth=320` inside a `blurred(...)` call: that string is the test's own input, not the output of `poster_url`.

## Done when

- [x] `rg -n "fillWidth=400" src/jellyfin/images.rs` prints the `poster_url` line
- [ ] On a Retina display the Movies grid posters look sharp

**Check:** the gate in `PLAN.md`.
