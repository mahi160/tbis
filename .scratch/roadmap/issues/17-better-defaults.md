# 17 — Better defaults

**What to build:** Change the defaults a first-time user gets. Each row is independent. The owner ticks the rows to apply; apply only ticked rows.

**Model:** haiku

**Blocked by:** Owner decision 2 in `PLAN.md` (tick rows below).

**Status:** done (runtime unverified). The owner ticked all four rows. The test `config_without_video_quality_loads_as_balanced` now expects High, and it is renamed `_as_high`.

**Read first:** `src/config.rs` (the `Default` impls and `fn defaults`), `src/jellyfin.rs` (`enum Sort`).

## Rows

| Apply | Setting | Today | Proposed | Why | Where |
| --- | --- | --- | --- | --- | --- |
| [x] | Hide episode spoilers | off | on | Unwatched Episode stills and descriptions show on Home and Series | `HideSpoilers`: replace `derive(Default)` with an `impl Default` returning `Self(true)`, as `ShowBackdrops` does |
| [x] | Movies and Series sort | Name | Date added | New arrivals first, like every streaming app | `enum Sort`: move `#[default]` to `DateAdded` |
| [x] | Video quality | Balanced | High | Only on machines with a capable GPU; leave Balanced when unsure | ticket 09's `VideoQuality`: move `#[default]` |
| [x] | Auto-skip intros | off | on | Fewer clicks; some users want to see intros | ticket 15's `AutoSkip` |

A changed default reaches existing users only for a setting they never touched: `config.json` stores a value once it was saved, and a stored value wins.

## Steps

1. For each ticked row, make the edit in the **Where** column.
2. Update the tests that assert the old default (`rg -n "default" src/config.rs src/jellyfin.rs` inside `mod tests`). Changing an expected value to the new default is the one allowed test edit in this ticket.

## Done when

- [x] Every ticked row is applied and no unticked row changed
- [ ] A fresh `config.json` (delete the dev one, run, quit) shows the new defaults

**Check:** the gate in `PLAN.md`.
