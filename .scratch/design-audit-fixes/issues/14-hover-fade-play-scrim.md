# 14 — Hover fade on the play scrim

**What to build:** The play scrim over posters, wide cards and Episode stills fades in when the pointer enters (about 150ms, ease-out) and fades out when it leaves, instead of snapping. With reduced motion on, it swaps instantly, as today. Nothing animates when a card first appears. The shadow lift and title hover color stay as they are.

**Blocked by:** 13 — Format the branch

**Status:** done (visual check pending)

- [ ] Hovering a poster, wide card or Episode still fades the scrim in, and leaving fades it out
- [ ] No fade or flash when cards first render, scroll into view, or load
- [ ] With reduced motion enabled, the scrim appears and disappears instantly
- [ ] The hover play button still plays and the card still opens (ticket 04 behaviour unchanged)
- [ ] Moving fast across a row of cards leaves no scrim stuck visible
- [x] `cargo test` passes
- [x] Checked in a confirmed-recompiled build (`Compiling tbis` in the output)
