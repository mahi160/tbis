# 35 — Player loads segments and the next Episode first, together

**What to build:** After a video starts, the Player fetches the Now Playing artwork, then the intro/credits segments, then the previous and next Episode, one after another. A slow artwork download delays the Skip Intro button and Autoplay; if the Episode ends before the next one is known, the Player closes instead of autoplaying. Fetch segments and Episodes together first, and the artwork last.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). Segments and adjacent Episodes are fetched concurrently before Now Playing artwork. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/player/mod.rs` (`fn load`, all of it).

## Steps

1. In `fn load` in `src/player/mod.rs`, keep everything up to and including this block:

   ```rust
   if this
       .update(cx, |this, cx| this.start(result, start_at, cx))
       .is_err()
       || !loaded
   {
       return;
   }
   ```

2. Replace everything after it, down to the end of the spawned `async move` block (the `if let Some(url) = artwork_url` block, the `match api.segments` block, the `let Some(series_id)` lines and the `match api.adjacent_episodes` block), with:

   ```rust
   // the skip button and Autoplay need these; neither waits for the other
   let episodes = async {
       match &series_id {
           Some(series_id) => Some(api.adjacent_episodes(series_id, &item_id).await),
           None => None,
       }
   };
   let (segments, episodes) = futures::join!(api.segments(&item_id), episodes);
   let Ok(()) = this.update(cx, |this, cx| {
       match segments {
           Ok(segments) => this.playback.segments = segments,
           // pre-10.10 servers 404 here: just no skip button
           Err(err) => crate::log::write(&format!("media segments lookup failed: {err}")),
       }
       match episodes {
           Some(Ok((previous, next))) => {
               this.playback.previous = previous;
               this.playback.next = next;
           }
           // no Autoplay; playback unaffected
           Some(Err(err)) => {
               crate::log::write(&format!("next episode lookup failed: {err}"))
           }
           None => {}
       }
       cx.notify();
   }) else {
       return;
   };
   // Now Playing artwork last: nothing on screen waits for it
   let Some(url) = artwork_url else {
       return;
   };
   match api.image_bytes(&url).await {
       Ok(bytes) => {
           this.update(cx, |this, _| {
               if let Some(now_playing) = &this.now_playing {
                   now_playing.set_artwork(&bytes);
               }
               this.playback.artwork = Some(bytes);
           })
           .ok();
       }
       Err(err) => crate::log::write(&format!("artwork fetch failed: {err}")),
   }
   ```

3. Update the doc comment on `fn load` to: "Fetches fresh play details, starts playback at `start_at` (else the server's resume point), then looks up the segments and neighbouring Episodes together, and the Now Playing artwork last."

The log messages are unchanged on purpose.

## Done when

- [ ] The Skip Intro button and the next/previous Episode buttons appear without waiting for the artwork
- [ ] Now Playing still shows the artwork
- [ ] A Movie (no Series) still loads its segments

**Check:** the gate in `PLAN.md`. Runtime: play an Episode that has an intro segment and confirm the Skip Intro button and the Next Episode button both work.
