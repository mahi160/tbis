# 32 — Per-Library sub-tabs on Movies and Series

**What to build:** On the Movies and Series pages, a row of sub-tabs: **All** (today's merged grid, the default), then one tab per Movie or Series Library of that kind, in server order. Picking a Library shows only that Library's items, with the same sort and grid. Hide the sub-tabs when the server has only one Library of that kind.

**Model:** sonnet

**Blocked by:** nothing. Ticket 28's `Api::libraries` and the `ParentId` query in `Api::latest_in` are the building blocks.

**Status:** needs-triage. The owner asked for it while deciding ticket 31. Open questions:
- Whether the sub-tabs appear in the Default layout too, or only in Modern.
- Whether the picked Library should be remembered between launches.

**Read first:** `CONTEXT.md` (**Library**), `AGENTS.md`, `src/library.rs` (`LibraryView`, `fn refresh`), `src/jellyfin/api.rs` (`fn library`, `fn latest_in`, `fn libraries`), `src/components/tabs.rs`.

## Steps

1. **API.** Give `Api::library` an optional `&Library` that adds `ParentId={id}`, as `latest_in` does. Test the query string.
2. **State.** `LibraryView` holds the Libraries of its kind and the picked one (`None` = All). Load the Libraries with the first refresh, and reload the grid when the pick changes.
3. **UI.** Build the sub-tabs from `components/tabs.rs` (extend it if it doesn't fit), placed above the grid.
4. **Words.** Update the **Library** entry in `CONTEXT.md`: the user can narrow Movies or Series to one Library.

## Done when

- [ ] Movies and Series show All plus one sub-tab per Library when there are two or more
- [ ] Picking a Library shows only its items; the sort still applies
- [ ] A server with one Library shows no sub-tabs
- [ ] Search still covers every Library

**Check:** the gate in `PLAN.md`, then screenshots of Movies and Series with a Library picked.
