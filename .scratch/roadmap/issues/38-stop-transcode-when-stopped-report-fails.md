# 38 — Stop the server's transcode even when the Stopped report fails

**What to build:** When the Player stops a transcoded stream it sends a Stopped report, then tells the server to end the transcode. If the report fails, the second call is skipped and the server keeps transcoding. On quit (cmd-Q) at the end of a video, no Stopped is sent at all. Make both paths always end the transcode.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). Stopped reports always attempt transcode deletion, including report failures. Quit sends Stopped before Played. A real HTTP regression test covers report failure, deletion failure and success. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/jellyfin/api.rs` (`pub async fn report`, `async fn stop_transcode`), `src/player/mod.rs` (`pub fn quit_report`), `src/player/playback.rs` (`fn end_current`, for the order to copy).

## Steps

1. In `pub async fn report` in `src/jellyfin/api.rs`, replace:

   ```rust
   self.post(path, body).await?;
   if let (Report::Stopped, Some(session)) = (report, &item.stream.transcode) {
       self.stop_transcode(session).await?;
   }
   Ok(())
   ```

   with:

   ```rust
   let reported = self.post(path, body).await;
   if let (Report::Stopped, Some(session)) = (report, &item.stream.transcode) {
       // even after a failed report: this is the only call that ends the server's ffmpeg job
       let stopped = self.stop_transcode(session).await;
       return reported.and(stopped);
   }
   reported
   ```

2. In `pub fn quit_report` in `src/player/mod.rs`, replace the body of the returned `async move` block with (Stopped first, then Played, the same order as `end_current`):

   ```rust
   let _ = api
       .report(Report::Stopped, &item, &session, time, paused)
       .await;
   if finished {
       let _ = api.set_played(&item.id, true).await;
   }
   ```

## Done when

- [x] `stop_transcode` runs for every Stopped report on a transcoded stream, whether or not the report succeeded
- [ ] Quitting at the end of a video sends Stopped and then marks it played

**Check:** the gate in `PLAN.md`.
