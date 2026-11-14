# 29 — Reusable paged row component

**What to build:** Home builds its horizontally scrolling rows, with their page arrows, inside `src/home.rs`. The Modern view needs the same row. Move the row's layout into `src/components/` so both can use it, with no visible change to Home.

**Model:** haiku

**Blocked by:** None.

**Status:** done. The element tree and every token call are unchanged; only the page buttons are now passed in as `AnyElement`. Release builds from before and after this ticket were screenshotted on the dev server: the rows, arrows and spacing match, and only the random page backdrop differs. Paging was not clicked.

**Read first:** `AGENTS.md` (UI rules), `src/home.rs` (`fn row`, `fn page`, `fn page_button`, `fn advance_paging`, `fn render`), `src/components/layout.rs` (`scroll_row`, `header_row`), `src/components/mod.rs`.

## Steps

1. Create `src/components/row.rs` and register it in `src/components/mod.rs` (`pub mod row;`, in alphabetical order).
2. Move the pure layout out of `HomeView::row` into a builder function:

   ```rust
   /// A titled row of cards that scrolls sideways, with previous and next page buttons.
   pub fn paged_row(
       id: impl Into<ElementId>,
       heading: impl IntoElement,
       scroll: &ScrollHandle,
       cards: Vec<AnyElement>,
       previous: AnyElement,
       next: AnyElement,
   ) -> Div
   ```

   It holds what `fn row` builds today: the header with the two buttons and the `scroll_row`. Keep every spacing and token call exactly as it is now.
3. `HomeView::row` keeps the parts that need `HomeView` state (which cards, the click handlers, the page buttons built by `page_button`) and calls `paged_row` for the layout.
4. Leave the paging state and logic (`page`, `advance_paging`, `scroll`, `paging`) in `home.rs`; only layout moves.

## Done when

- [x] `src/components/row.rs` exists and `home.rs` uses it
- [x] Home looks and pages exactly as before (arrows, spacing, scroll)
- [x] `home.rs` builds no row layout inline

**Check:** the gate in `PLAN.md`, then compare Home against the current build. Follow the build-freshness rule in `AGENTS.md`.
