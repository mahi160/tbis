# 32 — Per-Library sub-tabs on Movies and Series

**What to build:** On the Movies and Series pages, a row of sub-tabs: **All** (today's merged grid, the default), then one tab per Movie or Series Library of that kind, in server order. Picking a Library shows only that Library's items, with the same sort and grid. Hide the sub-tabs when the server has only one Library of that kind.

**Model:** sonnet

**Blocked by:** nothing. Ticket 28's `Api::libraries` and the `ParentId` query in `Api::latest_in` are the building blocks.

**Status:** done, partly runtime unverified.

**Owner answers:**
- **Setting:** the tabs don't depend on the Home layout. A new **Libraries** setting (Appearance: Merged, the default, or Separate) turns them on, and it affects only the Movies and Series pages.
- **Tabs:** Separate shows All first, then one tab per Library.
- **Memory:** the picked Library is not saved; each launch starts on All.

**Differences from the steps:**
- **API:** `Api::library` takes `within: Option<&Library>`. The query is built by a pure `library_query`, which is unit-tested.
- **Loading:** the Libraries load only under Separate, on each refresh. A picked Library that has disappeared, or is now the only one, falls back to All. Switching to Merged clears the pick.
- **Search:** it still covers every Library. Only Search's instant local matches come from the narrowed grid; the server search across all Libraries is merged in right after.
- **Review fixes:**
  - A failed load puts the highlighted tab back to the Library the grid shows.
  - A failed `/UserViews` call only hides the tabs; the grid still loads All.
  - A fallback to All scrolls to the top.
- **Tab bar:** it uses the existing `scrolling_tabs` and spans the page width, like the other tab strips.

**Checked on screen:** with a temporary local tweak so the tabs showed despite the dev server's single Library of each kind (reverted before commit):
- Movies and Series both showed All plus their Library.
- Picking the Movies Library narrowed the grid from 652 to 651 items, with the Date added sort kept.

**Not tried:** the Libraries setting switched live in Settings, and a server with two or more Libraries of one kind.

**Read first:** `CONTEXT.md` (**Library**), `AGENTS.md`, `src/library.rs` (`LibraryView`, `fn refresh`), `src/jellyfin/api.rs` (`fn library`, `fn latest_in`, `fn libraries`), `src/components/tabs.rs`.

## Steps

1. **API.** Give `Api::library` an optional `&Library` that adds `ParentId={id}`, as `latest_in` does. Test the query string.
2. **State.** `LibraryView` holds the Libraries of its kind and the picked one (`None` = All). Load the Libraries with the first refresh, and reload the grid when the pick changes.
3. **UI.** Build the sub-tabs from `components/tabs.rs` (extend it if it doesn't fit), placed above the grid.
4. **Words.** Update the **Library** entry in `CONTEXT.md`: the user can narrow Movies or Series to one Library.

## Done when

- [x] Movies and Series show All plus one sub-tab per Library when there are two or more
- [x] Picking a Library shows only its items; the sort still applies
- [x] A server with one Library shows no sub-tabs
- [x] Search still covers every Library

**Check:** the gate in `PLAN.md`, then screenshots of Movies and Series with a Library picked.
