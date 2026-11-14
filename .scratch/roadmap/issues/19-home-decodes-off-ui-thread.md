# 19 — Home decodes off the UI thread

**What to build:** `HomeView::refresh` awaits its four requests on the UI thread's executor, so their JSON is decoded there. Movies and Series already do this work in a background task. Do the same for Home.

**Model:** haiku

**Blocked by:** None.

**Status:** done (Home loads in a fresh debug build)

**Read first:** `src/home.rs` (`fn refresh`), `src/library.rs` (`fn refresh`, the `cx.background_spawn` block is the pattern).

## Steps

1. In `src/home.rs`, `fn refresh`, wrap the `futures::join!` in a background task and await it:

   ```rust
   let (resume, next_up, movies, series) = cx
       .background_spawn(async move {
           futures::join!(
               api.resume(PER_ROW),
               api.next_up_all(PER_ROW * 2),
               api.latest_unplayed(Kind::Movie, PER_ROW * 2),
               api.latest_unplayed(Kind::Series, PER_ROW),
           )
       })
       .await;
   ```

2. Keep the comment about over-fetching and everything after the join unchanged.

## Done when

- [x] The four requests and their decoding run inside `background_spawn`
- [x] Home shows the same rows as before

**Check:** the gate in `PLAN.md`, then open the app and confirm Home loads.
