# 18 — Tabs reload only when stale

**What to build:** Every tab press refetches that tab: four requests for Home, the whole Library for Movies or Series. Refetch on a tab press only when the data is older than a minute or something changed (a video was played, a detail page was left).

**Model:** haiku

**Blocked by:** None.

**Status:** done, runtime unverified. `mark_stale` also cancels an in-flight load, so a hidden tab's load started before the change can't mark itself fresh.

**Read first:** `src/app.rs` (`Main::refresh_visible`, `fn select_tab`, `fn close_player`, the `Nav::Back` arm), `src/home.rs` (`HomeView`, `fn refresh`), `src/library.rs` (`LibraryView`, `fn refresh`, `fn set_sort`).

## Steps

1. In both `HomeView` and `LibraryView`, add a field `loaded_at: Option<std::time::Instant>` (initial `None`).
2. In each `fn refresh`, in the success arm where the fresh data is stored, set `this.loaded_at = Some(std::time::Instant::now());`.
3. Add to both views:

   ```rust
   /// Marks the data out of date, so the next tab visit reloads it.
   pub fn mark_stale(&mut self) {
       self.loaded_at = None;
   }

   /// Reloads unless the data is under a minute old.
   pub fn refresh_if_stale(&mut self, cx: &mut Context<Self>) {
       const FRESH_FOR: std::time::Duration = std::time::Duration::from_secs(60);
       if self.loaded_at.is_some_and(|at| at.elapsed() < FRESH_FOR) {
           return;
       }
       self.refresh(cx);
   }
   ```

4. In `src/app.rs`, `Main::refresh_visible` runs after something may have changed (Player closed, detail page left). At its top, mark all three views stale:

   ```rust
   self.home.update(cx, |view, _| view.mark_stale());
   self.movies.update(cx, |view, _| view.mark_stale());
   self.series.update(cx, |view, _| view.mark_stale());
   ```

5. Add a sibling method on `Main`:

   ```rust
   /// Tab visit: reloads the tab only when its data is stale.
   fn refresh_tab_if_stale(&self, cx: &mut App) {
       match self.tab {
           Tab::Movies => self.movies.update(cx, |view, cx| view.refresh_if_stale(cx)),
           Tab::Series => self.series.update(cx, |view, cx| view.refresh_if_stale(cx)),
           Tab::Home => self.home.update(cx, |view, cx| view.refresh_if_stale(cx)),
       }
   }
   ```

6. In `fn select_tab`, replace `main.refresh_visible(cx);` with `main.refresh_tab_if_stale(cx);`. `select_tab` clears `details` first, so the tab is what is visible. Leave `close_player` and `Nav::Back` calling `refresh_visible`.
7. Leave `set_sort` and the retry buttons calling `refresh` directly.

## Done when

- [x] Switching Home → Movies → Home within a minute sends no requests on the second Home visit
- [x] After closing the Player, the next visit to any tab reloads it
- [x] After a minute, a tab visit reloads
- [x] Changing the sort still reloads at once

**Check:** the gate in `PLAN.md`. Runtime: watch the Jellyfin server's request log, or add a temporary `crate::log::write` in `refresh`, and remove it before finishing.
