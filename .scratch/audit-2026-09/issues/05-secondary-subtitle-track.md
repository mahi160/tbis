# 05 — Secondary subtitle track

**What to build:** The existing track picker only ever sets one subtitle track via `select_track`/`sid`. Add a second slot using mpv's `secondary-sid`, so a user can show two subtitle tracks at once (e.g. original + translated) — a second row in the track picker UI already built, not new plumbing (`docs/audit-2026-09.html` §3, §5, §8).

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Track picker offers a second subtitle row (including "Off") alongside the primary subtitle row
- [ ] Both subtitle tracks render simultaneously when both are set, at their existing styling
- [ ] Setting the secondary track to "Off" leaves the primary track unaffected
- [ ] PiP reflects the same secondary-track selection
