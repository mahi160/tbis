# 28 — Jellyfin: Libraries and per-Library queries

**What to build:** The Jellyfin layer always queries every Library merged together. The Modern view needs the list of Libraries and the newest items of one Library, plus a logo image for the hero. Add those three calls, with tests, and no UI.

**Model:** sonnet

**Blocked by:** None. (Layout-independent, so it can be done while the Modern layout decision is open.)

**Status:** ready-for-agent

**Read first:** `CONTEXT.md` (the word is **Library**), `src/jellyfin.rs` or `src/jellyfin/` after ticket 26 (`Api::items`, `Api::latest_unplayed`, `Api::backdrop_url`, `Item`, the test module that decodes sample JSON), and the Jellyfin API reference for the endpoints below.

## Steps

1. Confirm the endpoint shapes before writing code. Use the context7 docs tool for Jellyfin's API, or `curl` a real server with the token from the dev `config.json`. Record the confirmed request and one trimmed response in this ticket. Expected, to be confirmed:
   - Libraries: `GET /UserViews?userId={id}` returning `Items` with `Id`, `Name`, `CollectionType` (`movies`, `tvshows`, and others).
   - Items of one Library: the existing `/Items` query with `ParentId={library id}`.
   - Logo: `/Items/{id}/Images/Logo?maxWidth=...&tag={tag}` with the tag from the item's `ImageTags.Logo`.
2. Add a type:

   ```rust
   /// A Jellyfin media folder that holds Movies or Series.
   pub struct Library {
       pub id: String,
       pub name: String,
       pub kind: Kind,
   }
   ```

3. Add `Api::libraries(&self) -> Result<Vec<Library>>`, keeping only Libraries whose collection type is movies or series, in server order.
4. Add `Api::latest_in(&self, library: &Library, limit: usize) -> Result<Vec<Item>>`: newest first, reusing the query builder `latest_unplayed` uses, plus `ParentId`.
5. Add `Api::logo_url(&self, item: &Item) -> Option<String>`, following `backdrop_url`. Request the image tag field the same way the backdrop tag is requested.
6. Tests, following the existing decode tests: a sample Libraries response decodes to the right kinds and drops other collection types; `logo_url` is `None` without a tag and a full URL with one.

## Done when

- [ ] The three confirmed request and response shapes are recorded in this ticket
- [ ] `libraries`, `latest_in` and `logo_url` exist with tests
- [ ] No existing call changed behaviour

**Check:** the gate in `PLAN.md`. The new functions are unused until ticket 31; add `#[allow(dead_code)]` with the comment `// used by the Modern view (roadmap 31)` so Clippy passes, and remove it in ticket 31.
