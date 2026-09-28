//! The current Episode/Movie's state and its server reports.

use std::sync::Arc;

use futures::StreamExt as _;
use futures::channel::mpsc::{self, UnboundedSender};
use gpui_kit::*;

use crate::jellyfin::{Api, Item, Kind, PlaybackItem, Report, Segment, SegmentKind};
use crate::mpv::{MpvEvent, Track, TrackKind};
use crate::pip::PipEvent;

use super::PlayerView;

/// Seconds left in an Episode when the next one is offered.
const UP_NEXT_AT: f64 = 30.;
/// Redraws throttle to this step, shared by the mpv and PiP position streams.
const REDRAW_STEP: f64 = 0.25;
/// Seconds before a segment's end at which its skip button hides.
const SEGMENT_END_MARGIN: f64 = 0.5;

/// Server update queued by the Player; sent in order, even after it closes.
pub(super) enum Queued {
    Report {
        report: Report,
        item: Arc<PlaybackItem>,
        seconds: f64,
        paused: bool,
    },
    Played(Arc<PlaybackItem>),
}

/// Everything tied to the Episode/Movie currently loaded; replaced whole by
/// `Playback::new` on Autoplay so no field can be missed on reset (former bug:
/// `play_next` cleared 14 fields by hand and skipped `error`).
pub(super) struct Playback {
    /// Series title for an Episode, else the item's own name.
    pub(super) title: SharedString,
    /// `S01E01 · Name` under an Episode's series title.
    pub(super) subtitle: Option<SharedString>,
    pub(super) item: Option<Arc<PlaybackItem>>,
    /// Episode that Autoplay continues with.
    pub(super) next: Option<Item>,
    /// Episode before this one, for the previous button.
    pub(super) previous: Option<Item>,
    pub(super) next_cancelled: bool,
    pub(super) started: bool,
    /// Guards external `sub-add` against a repeat `FileLoaded` for the same item.
    pub(super) subs_added: bool,
    pub(super) finished: bool,
    pub(super) time: f64,
    /// Time last drawn; throttles redraws to `REDRAW_STEP`.
    pub(super) shown_time: f64,
    pub(super) duration: f64,
    pub(super) scrubbing: Option<f64>,
    pub(super) tracks: Vec<Track>,
    /// Guards the remembered-pick apply to once per item, the first time the track
    /// list is non-empty (see `PlayerView::apply_remembered_tracks`).
    pub(super) tracks_applied: bool,
    pub(super) error: Option<SharedString>,
    /// Intro/credits ranges; empty until fetched or on servers without media segments.
    pub(super) segments: Vec<Segment>,
    /// Now Playing artwork: Episode thumb or poster, fetched after load.
    pub(super) artwork_url: Option<String>,
    pub(super) artwork: Option<Vec<u8>>,
    pub(super) reports: UnboundedSender<Queued>,
    /// Session id sent with every report for this item; also used for the
    /// best-effort report fired from `on_app_quit`.
    pub(super) play_session_id: String,
    pub(super) _load: Task<()>,
}

impl Playback {
    pub(super) fn new(item: &Item, api: &Api, cx: &mut Context<PlayerView>) -> Self {
        let (title, subtitle) = titles(item);
        let play_session_id = uuid::Uuid::new_v4().simple().to_string();
        let reports = spawn_reporter(api.clone(), play_session_id.clone(), cx);
        Self {
            title,
            subtitle,
            item: None,
            next: None,
            previous: None,
            next_cancelled: false,
            started: false,
            subs_added: false,
            finished: false,
            time: 0.,
            shown_time: 0.,
            duration: 0.,
            scrubbing: None,
            tracks: Vec::new(),
            tracks_applied: false,
            error: None,
            segments: Vec::new(),
            artwork_url: api.wide_image_url(item).or_else(|| api.poster_url(item)),
            artwork: None,
            reports,
            play_session_id,
            _load: Task::ready(()),
        }
    }
}

impl PlayerView {
    pub(super) fn report(&self, report: Report) {
        if let Some(item) = &self.playback.item {
            let _ = self.playback.reports.unbounded_send(Queued::Report {
                report,
                item: item.clone(),
                seconds: self.playback.time,
                paused: self.paused && self.pip.is_none(), // Player pauses while PiP plays
            });
        }
    }

    /// Time bookkeeping shared by the mpv and PiP position streams. Returns whether
    /// the `REDRAW_STEP` threshold was crossed, i.e. whether the UI should redraw.
    fn update_time(&mut self, time: f64) -> bool {
        self.playback.time = time;
        if (time - self.playback.shown_time).abs() < REDRAW_STEP {
            return false;
        }
        self.playback.shown_time = time;
        true
    }

    pub(super) fn on_mpv(&mut self, event: MpvEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            MpvEvent::TimePos(time) => {
                if !self.update_time(time) {
                    return;
                }
                self.sync_seek_bar(window, cx);
            }
            // resume jump arrives before duration is known
            MpvEvent::Duration(duration) => {
                self.playback.duration = duration;
                self.sync_seek_bar(window, cx);
            }
            MpvEvent::Pause(paused) => {
                self.paused = paused;
                if self.playback.started {
                    self.report(Report::Progress);
                }
                self.show_controls(window, cx);
            }
            MpvEvent::Volume(volume) => {
                self.volume = volume;
                let shown = self.volume_slider.read(cx).value().end() as f64;
                if (shown - volume).abs() >= 0.5 {
                    self.volume_slider
                        .update(cx, |s, cx| s.set_value(volume as f32, window, cx));
                }
            }
            MpvEvent::Mute(muted) => self.muted = muted,
            MpvEvent::Tracks(tracks) => {
                self.playback.tracks = tracks;
                // The embedded track list can turn up before `FileLoaded`'s sub-add for
                // this item's external subtitles lands; applying now would pick a
                // language/remembered subtitle from a list that's still missing them.
                let subs_pending = !self.playback.subs_added
                    && self
                        .playback
                        .item
                        .as_ref()
                        .is_some_and(|item| !self.api.external_subtitles(item).is_empty());
                if !self.playback.tracks_applied
                    && !self.playback.tracks.is_empty()
                    && !subs_pending
                {
                    self.playback.tracks_applied = true;
                    self.apply_remembered_tracks();
                }
            }
            MpvEvent::FileLoaded => {
                if !self.playback.started {
                    self.playback.started = true;
                    self.report(Report::Start);
                }
                // sub-add before FILE_LOADED races the core opening the file (mpv has no
                // current file to attach the track to yet); deferred to here instead.
                // Guarded: a repeat FILE_LOADED for this item would otherwise re-add every
                // external subtitle and double up the track menu.
                if !self.playback.subs_added
                    && let (Some(mpv), Some(item)) = (&self.mpv, &self.playback.item)
                {
                    self.playback.subs_added = true;
                    for sub in self.api.external_subtitles(item) {
                        let _ = mpv.add_subtitle(&sub.url, &sub.title, &sub.lang, sub.is_default);
                    }
                }
            }
            MpvEvent::EndFile(Ok(true)) => {
                self.playback.finished = true;
                self.playback.time = self.playback.duration;
                match self
                    .playback
                    .next
                    .clone()
                    .filter(|_| !self.playback.next_cancelled)
                {
                    Some(next) => self.play_next(next, window, cx),
                    None => self.close(window, cx),
                }
                return;
            }
            MpvEvent::EndFile(Ok(false)) => {}
            MpvEvent::EndFile(Err(err)) => {
                self.playback.error = Some(format!("Playback failed: {err}").into())
            }
        }
        cx.notify();
    }

    pub(super) fn on_pip(&mut self, event: PipEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            PipEvent::Position(time) => {
                if self.update_time(time) {
                    self.sync_seek_bar(window, cx);
                    cx.notify();
                }
            }
            PipEvent::Ended(time) => {
                self.pip = None;
                if let Some(mpv) = &self.mpv {
                    let _ = mpv.seek(time);
                    let _ = mpv.set_pause(false);
                }
                self.playback.time = time;
                self.sync_seek_bar(window, cx);
                self.show_controls(window, cx);
            }
        }
    }

    /// Segment the playhead is in, while the in-window Player plays without error.
    pub(super) fn active_segment(&self) -> Option<Segment> {
        if self.pip.is_some() || self.playback.error.is_some() {
            return None;
        }
        let time = self.playback.time;
        // margin: a skip lands on `end`, so don't linger on the button there
        self.playback
            .segments
            .iter()
            .copied()
            .find(|s| s.start <= time && time < s.end - SEGMENT_END_MARGIN)
    }

    /// Seeks past the current segment. Skipping credits with Autoplay pending plays the
    /// next Episode instead, counting this one watched like Play now.
    pub(super) fn skip_segment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(segment) = self.active_segment() else {
            return;
        };
        if segment.kind == SegmentKind::Outro
            && !self.playback.next_cancelled
            && let Some(next) = self.playback.next.clone()
        {
            self.playback.finished = true;
            self.play_next(next, window, cx);
            return;
        }
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.seek(segment.end);
        }
        self.refocus(window, cx);
    }

    /// Next Episode while its card should show: last 30s, not cancelled, not in PiP.
    pub(super) fn up_next(&self) -> Option<&Item> {
        let near_end = self.playback.duration > 0.
            && self.playback.duration - self.playback.time <= UP_NEXT_AT;
        if near_end
            && !self.playback.next_cancelled
            && self.playback.error.is_none()
            && self.pip.is_none()
        {
            self.playback.next.as_ref()
        } else {
            None
        }
    }

    /// Loads the next Episode. Caller sets `finished` beforehand: the EOF path and
    /// Play now/Enter count the current Episode watched, the skip button does not.
    pub(super) fn play_next(&mut self, next: Item, window: &mut Window, cx: &mut Context<Self>) {
        self.end_current();
        self.pip = None;
        self._pip = Task::ready(()); // drop stale PiP events
        if let Some(mpv) = &self.mpv {
            // stop old file during fetch; pause would carry over
            let _ = mpv.stop();
            let _ = mpv.set_pause(false);
        }
        self.playback = Playback::new(&next, &self.api, cx);
        self.seek.update(cx, |s, cx| s.set_value(0., window, cx));
        self.load(next.id, cx);
        self.refocus(window, cx);
    }

    /// Reports stop, and played when finished, for the current item.
    pub(super) fn end_current(&mut self) {
        if self.playback.started {
            self.report(Report::Stopped);
        }
        // explicit: Stopped near end alone depends on server's resume thresholds
        if let (true, Some(item)) = (self.playback.finished, &self.playback.item) {
            let _ = self
                .playback
                .reports
                .unbounded_send(Queued::Played(item.clone()));
        }
    }

    pub(super) fn set_speed(&mut self, speed: f64) {
        if let Some(mpv) = self.active_mpv()
            && mpv.set_speed(speed).is_ok()
        {
            self.speed = speed;
        }
    }

    pub(super) fn select_track(&mut self, kind: TrackKind, id: Option<i64>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.select_track(kind, id);
        }
        self.remember_track(kind, id);
    }

    /// Records a manual track pick: this exact item's choice, plus (audio always;
    /// subtitle only when turning one on) the language -- scoped to the Series for an
    /// Episode so it can't drag a pick into an unrelated show, else the app-wide
    /// default (a Movie has no series to scope to). Not called for the automatic apply
    /// below -- that would just write back what it read.
    fn remember_track(&mut self, kind: TrackKind, id: Option<i64>) {
        let Some(item) = self.playback.item.clone() else {
            return;
        };
        let lang = id.and_then(|id| {
            self.playback
                .tracks
                .iter()
                .find(|t| t.kind == kind && t.id == id)
                .and_then(|t| t.lang.clone())
        });

        let entry = self.track_prefs.memory.entry(item.id.clone()).or_default();
        match kind {
            TrackKind::Audio => entry.audio = id,
            TrackKind::Subtitle => {
                entry.subtitle = id;
                entry.subtitle_off = id.is_none();
            }
        }

        let pref = match &item.series_id {
            Some(series_id) => self
                .track_prefs
                .series
                .entry(series_id.clone())
                .or_default(),
            None => &mut self.track_prefs.global,
        };
        match kind {
            TrackKind::Audio => {
                if let Some(lang) = lang {
                    pref.audio_lang = Some(lang);
                }
            }
            TrackKind::Subtitle => {
                pref.subtitles_enabled = Some(id.is_some());
                if let Some(lang) = lang {
                    pref.subtitle_lang = Some(lang);
                }
            }
        }
    }

    /// Applies this item's exact remembered pick if there is one, else a language:
    /// this Series' own pick, then Settings' preference, then the app-wide last pick
    /// (a Movie, or a Series never configured, skips the first). Matches mpv's own
    /// current selection otherwise, i.e. does nothing.
    pub(super) fn apply_remembered_tracks(&mut self) {
        let Some(item) = self.playback.item.clone() else {
            return;
        };
        let remembered = self.track_prefs.memory.get(&item.id).copied();
        let series_pref = item
            .series_id
            .as_ref()
            .and_then(|id| self.track_prefs.series.get(id))
            .cloned();
        let (preferred, global) = (&self.language, &self.track_prefs.global);
        let audio_lang = series_pref
            .as_ref()
            .and_then(|p| p.audio_lang.clone())
            .or_else(|| preferred.audio_lang.clone())
            .or_else(|| global.audio_lang.clone());
        let subtitles_enabled = series_pref
            .as_ref()
            .and_then(|p| p.subtitles_enabled)
            .or(preferred.subtitles_enabled)
            .or(global.subtitles_enabled)
            .unwrap_or(false);
        // language from the same scope that decided subtitles are on
        let subtitle_lang = series_pref
            .as_ref()
            .and_then(|p| p.subtitle_lang.clone())
            .or_else(|| {
                preferred
                    .subtitles_enabled
                    .and(preferred.subtitle_lang.clone())
            })
            .or_else(|| global.subtitle_lang.clone());

        let audio = remembered
            .and_then(|r| r.audio)
            .filter(|id| self.has_track(TrackKind::Audio, *id))
            .or_else(|| self.track_by_lang(TrackKind::Audio, audio_lang.as_deref()));
        if let Some(mpv) = self.active_mpv()
            && let Some(id) = audio
        {
            let _ = mpv.select_track(TrackKind::Audio, Some(id));
        }

        if let Some(r) = remembered {
            if r.subtitle_off {
                if let Some(mpv) = self.active_mpv() {
                    let _ = mpv.select_track(TrackKind::Subtitle, None);
                }
                return;
            }
            if let Some(id) = r
                .subtitle
                .filter(|id| self.has_track(TrackKind::Subtitle, *id))
            {
                if let Some(mpv) = self.active_mpv() {
                    let _ = mpv.select_track(TrackKind::Subtitle, Some(id));
                }
                return;
            }
        }
        if subtitles_enabled
            && let Some(id) = self.track_by_lang(TrackKind::Subtitle, subtitle_lang.as_deref())
            && let Some(mpv) = self.active_mpv()
        {
            let _ = mpv.select_track(TrackKind::Subtitle, Some(id));
        }
    }

    fn has_track(&self, kind: TrackKind, id: i64) -> bool {
        self.playback
            .tracks
            .iter()
            .any(|t| t.kind == kind && t.id == id)
    }

    fn track_by_lang(&self, kind: TrackKind, lang: Option<&str>) -> Option<i64> {
        let lang = lang?;
        self.playback
            .tracks
            .iter()
            .find(|t| {
                t.kind == kind
                    && t.lang
                        .as_deref()
                        .is_some_and(|l| crate::settings::same_language(l, lang))
            })
            .map(|t| t.id)
    }

    /// Selects the next track of `kind`, wrapping. Subtitle also cycles through "off".
    pub(super) fn cycle_track(&mut self, kind: TrackKind) {
        let tracks: Vec<&Track> = self
            .playback
            .tracks
            .iter()
            .filter(|t| t.kind == kind)
            .collect();
        if tracks.is_empty() {
            return;
        }
        let current = tracks.iter().position(|t| t.selected);
        let id = match kind {
            TrackKind::Subtitle => {
                let next = current.map_or(0, |i| i + 1);
                tracks.get(next).map(|t| t.id)
            }
            TrackKind::Audio => {
                let next = current.map_or(0, |i| (i + 1) % tracks.len());
                Some(tracks[next].id)
            }
        };
        self.select_track(kind, id);
    }

    /// Jellyfin's own chapter list (not mpv's embedded one -- present on more files,
    /// carries a name); the one whose start is at or before `self.playback.time`.
    pub(super) fn chapter_index(&self) -> Option<usize> {
        let chapters = &self.playback.item.as_ref()?.chapters;
        if chapters.is_empty() {
            return None;
        }
        Some(
            chapters
                .iter()
                .rposition(|c| c.start_seconds() <= self.playback.time)
                .unwrap_or(0),
        )
    }

    /// `direction`: -1 previous, 1 next. More than 3s into the current chapter,
    /// "previous" restarts it instead of skipping to the one before (DVD-player convention).
    pub(super) fn seek_chapter(&mut self, direction: i32) {
        let Some(item) = self.playback.item.clone() else {
            return;
        };
        if item.chapters.is_empty() {
            return;
        }
        let current = self.chapter_index().unwrap_or(0);
        let restart =
            direction < 0 && self.playback.time - item.chapters[current].start_seconds() > 3.;
        let target = if restart {
            current
        } else {
            (current as i32 + direction).clamp(0, item.chapters.len() as i32 - 1) as usize
        };
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.seek(item.chapters[target].start_seconds());
        }
    }
}

/// Series name over `S01E01 · Name` for Episodes; else just the name.
fn titles(item: &Item) -> (SharedString, Option<SharedString>) {
    let name = item.display_name().to_string();
    match (
        item.kind == Kind::Episode,
        &item.series_name,
        item.parent_index_number,
        item.index_number,
    ) {
        (true, Some(series), Some(season), Some(episode)) => (
            series.clone().into(),
            Some(
                format!(
                    "{} \u{b7} {name}",
                    crate::jellyfin::episode_code(season, episode)
                )
                .into(),
            ),
        ),
        _ => (name.into(), None),
    }
}

/// Sends reports one by one so the server sees them in order.
fn spawn_reporter(
    api: Api,
    play_session_id: String,
    cx: &mut Context<PlayerView>,
) -> UnboundedSender<Queued> {
    let (tx, mut rx) = mpsc::unbounded::<Queued>();
    cx.background_spawn(async move {
        while let Some(queued) = rx.next().await {
            let result = match queued {
                Queued::Report {
                    report,
                    item,
                    seconds,
                    paused,
                } => {
                    api.report(report, &item, &play_session_id, seconds, paused)
                        .await
                }
                Queued::Played(item) => api.set_played(&item.id, true).await,
            };
            if let Err(err) = result {
                eprintln!("playback report failed: {err}");
            }
        }
    })
    .detach();
    tx
}
