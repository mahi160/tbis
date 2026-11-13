//! The current Episode/Movie's state and its server reports.

use std::sync::Arc;

use futures::StreamExt as _;
use futures::channel::mpsc::{self, UnboundedSender};
use gpui_kit::*;

use crate::jellyfin::{Api, Item, Kind, PlaybackItem, Report, Segment, SegmentKind};
use crate::mpv::{FileEvent, MpvEvent, PlayerEvent, Track, TrackKind};

use super::PlayerView;
use super::controls::StaticInfo;

/// Seconds left in an Episode when the next one is offered.
const UP_NEXT_AT: f64 = 30.;
/// Redraws throttle to this step; below the mpv event throttle so slow speeds stay smooth.
const REDRAW_STEP: f64 = 0.1;
/// Seconds before a segment's end at which its skip button hides.
const SEGMENT_END_MARGIN: f64 = 0.5;

/// When remembered tracks may be applied for one load: once, after every external
/// subtitle it adds has settled (added or failed).
#[derive(Default)]
struct SubtitleReadiness {
    /// External `sub-add`s sent; a repeat `FileLoaded` must not add them again.
    queued: bool,
    pending: usize,
    applied: bool,
}

impl SubtitleReadiness {
    /// Starts `count` additions; `false` when this load already queued its subtitles.
    fn begin(&mut self, count: usize) -> bool {
        if self.queued {
            return false;
        }
        self.queued = true;
        self.pending = count;
        true
    }

    fn settle(&mut self) {
        self.pending = self.pending.saturating_sub(1);
    }

    /// `expects_external`: the item has external subtitles. The embedded track list can
    /// arrive before they are even queued; applying then would pick from a list missing them.
    fn take_ready(&mut self, has_tracks: bool, expects_external: bool) -> bool {
        let waiting = self.pending > 0 || (!self.queued && expects_external);
        if !waiting && has_tracks && !self.applied {
            self.applied = true;
            true
        } else {
            false
        }
    }
}

/// Time-driven overlay state; a time update redraws a hidden-controls Player only when it changes.
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct TimeOverlays {
    segment: Option<SegmentKind>,
    up_next_left: Option<u64>,
}

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
    /// The library item being played; reloaded from when the bitrate cap changes.
    pub(super) source: Item,
    /// Position this load starts at (`None`: the server's resume point); a reload
    /// before the first time update keeps it rather than restarting at 0.
    pub(super) start_at: Option<f64>,
    /// Series title for an Episode, else the item's own name.
    pub(super) title: SharedString,
    /// `S01E01 · Name` under an Episode's series title.
    pub(super) subtitle: Option<SharedString>,
    pub(super) item: Option<Arc<PlaybackItem>>,
    pub(super) load: Option<i64>,
    /// Episode that Autoplay continues with.
    pub(super) next: Option<Item>,
    /// Episode before this one, for the previous button.
    pub(super) previous: Option<Item>,
    pub(super) next_cancelled: bool,
    pub(super) started: bool,
    subtitle_readiness: SubtitleReadiness,
    pub(super) finished: bool,
    pub(super) time: f64,
    /// Time last drawn; throttles redraws to `REDRAW_STEP`.
    pub(super) shown_time: f64,
    pub(super) duration: f64,
    pub(super) scrubbing: Option<f64>,
    /// Time-driven overlays last drawn; see `PlayerView::time_overlays`.
    pub(super) overlays: TimeOverlays,
    /// Playback info's file/track values, read once; cleared when the tracks change.
    pub(super) info_static: Option<StaticInfo>,
    pub(super) tracks: Vec<Track>,
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
            source: item.clone(),
            start_at: None,
            title,
            subtitle,
            item: None,
            load: None,
            next: None,
            previous: None,
            next_cancelled: false,
            started: false,
            subtitle_readiness: SubtitleReadiness::default(),
            finished: false,
            time: 0.,
            shown_time: 0.,
            duration: 0.,
            scrubbing: None,
            overlays: TimeOverlays::default(),
            info_static: None,
            tracks: Vec::new(),
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
                paused: self.paused,
            });
        }
    }

    /// Time bookkeeping for mpv's position stream. Returns whether
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
        if !event.is_current(self.playback.load) {
            return;
        }
        let event = match event {
            MpvEvent::Player(event) => {
                self.on_player_event(event, window, cx);
                return;
            }
            MpvEvent::File { event, .. } => event,
        };
        match event {
            FileEvent::TimePos(time) => {
                if !self.update_time(time) {
                    return;
                }
                self.sync_seek_bar(window, cx);
                self.sync_now_playing(window, cx);
                // with the controls hidden only the skip button and up-next card show time
                let overlays = self.time_overlays();
                if !self.controls_visible && overlays == self.playback.overlays {
                    return;
                }
                self.playback.overlays = overlays;
            }
            // resume jump arrives before duration is known
            FileEvent::Duration(duration) => {
                self.playback.duration = duration;
                self.sync_seek_bar(window, cx);
            }
            FileEvent::Pause(paused) => {
                self.paused = paused;
                if self.playback.started {
                    self.report(Report::Progress);
                }
                self.show_controls(window, cx);
            }
            FileEvent::CoreIdle(idle) => self.core_idle = idle,
            FileEvent::Tracks(tracks) => {
                self.playback.tracks = tracks;
                self.playback.info_static = None;
                self.apply_tracks_if_ready();
            }
            FileEvent::FileLoaded => {
                if !self.playback.started {
                    if let Some(mpv) = &self.mpv {
                        let delay = self
                            .track_prefs
                            .playback
                            .get(&self.playback.source.id)
                            .and_then(|prefs| prefs.subtitle_delay)
                            .unwrap_or(0.);
                        for (name, value) in [
                            ("speed", self.speed),
                            ("sub-delay", delay),
                            ("audio-delay", self.audio_delay),
                        ] {
                            if let Err(err) = mpv.set_property(name, &value.to_string()) {
                                crate::log::write(&format!("failed to restore {name}: {err}"));
                            }
                        }
                    }
                    self.playback.started = true;
                    self.report(Report::Start);
                }
                // sub-add before FILE_LOADED races the core opening the file (mpv has no
                // current file to attach the track to yet); deferred to here instead.
                // Guarded by `begin`: a repeat FILE_LOADED for this item would otherwise
                // re-add every external subtitle and double up the track menu.
                if let (Some(mpv), Some(item), Some(load)) =
                    (&self.mpv, &self.playback.item, self.playback.load)
                    && let subtitles = self.api.external_subtitles(item)
                    && self.playback.subtitle_readiness.begin(subtitles.len())
                {
                    for sub in subtitles {
                        if let Err(err) = mpv.add_subtitle(
                            &sub.url,
                            &sub.title,
                            &sub.lang,
                            sub.is_default,
                            load,
                        ) {
                            crate::log::write(&format!("external subtitle addition failed: {err}"));
                            self.playback.subtitle_readiness.settle();
                        }
                    }
                    self.apply_tracks_if_ready();
                }
            }
            FileEvent::SubtitleAdded(result) => {
                if let Err(err) = result {
                    crate::log::write(&format!("external subtitle addition failed: {err}"));
                }
                self.playback.subtitle_readiness.settle();
                self.apply_tracks_if_ready();
            }
            FileEvent::EndFile(Ok(true)) => {
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
            FileEvent::EndFile(Ok(false)) => {}
            FileEvent::EndFile(Err(err)) => {
                self.playback.error = Some(format!("Playback failed: {err}").into())
            }
        }
        cx.notify();
    }

    fn on_player_event(&mut self, event: PlayerEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            PlayerEvent::Volume(volume) => {
                self.volume = volume;
                let shown = self.volume_slider.read(cx).value().end() as f64;
                if (shown - volume).abs() >= 0.5 {
                    self.volume_slider
                        .update(cx, |s, cx| s.set_value(volume as f32, window, cx));
                }
            }
            PlayerEvent::Mute(muted) => self.muted = muted,
            PlayerEvent::RenderFailed(err) => {
                self.playback.error = Some(format!("Playback failed: {err}").into());
                // video is gone for good; mpv would otherwise play on, unseen
                self.mpv = None;
                self.drop_pip(cx);
            }
        }
        cx.notify();
    }

    fn apply_tracks_if_ready(&mut self) {
        let expects_external = (self.playback.item.as_ref())
            .is_some_and(|item| !self.api.external_subtitles(item).is_empty());
        if self
            .playback
            .subtitle_readiness
            .take_ready(!self.playback.tracks.is_empty(), expects_external)
        {
            self.apply_remembered_tracks();
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

    /// What the time-driven overlays show: the skip button's segment and the up-next
    /// card's whole-second countdown.
    fn time_overlays(&self) -> TimeOverlays {
        TimeOverlays {
            segment: self.active_segment().map(|s| s.kind),
            up_next_left: self
                .up_next()
                .map(|_| (self.playback.duration - self.playback.time).max(0.).ceil() as u64),
        }
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
        // no mpv: keep its error rather than a blank Player that never loads
        if self.mpv.is_none() {
            return;
        }
        self.end_current();
        if let Some(mpv) = &self.mpv {
            // stop old file during fetch; pause would carry over
            let _ = mpv.stop();
            let _ = mpv.set_pause(false);
        }
        self.paused = false;
        self.playback = Playback::new(&next, &self.api, cx);
        self.restore_item_preferences();
        self.seek.update(cx, |s, cx| s.set_value(0., window, cx));
        self.load(next.id, None, cx);
        self.refocus(window, cx);
    }

    /// Caps the stream at `mbps` (`None`: the original file) for the rest of this
    /// Player, reloading the current item from where it is. Pause carries over.
    pub(super) fn set_max_bitrate(&mut self, mbps: Option<u32>, cx: &mut Context<Self>) {
        // no mpv: keep its "Cannot start player" error rather than a blank reload
        if mbps == self.max_bitrate_mbps || self.mpv.is_none() {
            return;
        }
        self.max_bitrate_mbps = mbps;
        let prefs = self
            .track_prefs
            .playback
            .entry(self.playback.source.id.clone())
            .or_default();
        prefs.bitrate_set = true;
        prefs.max_bitrate_mbps = mbps;
        let item = self.playback.source.clone();
        let at = if self.playback.time > 0. {
            Some(self.playback.time)
        } else {
            self.playback.start_at
        };
        // Stopped also ends the server's transcode for the old stream
        self.end_current();
        if let Some(mpv) = &self.mpv {
            let _ = mpv.stop();
        }
        self.playback = Playback::new(&item, &self.api, cx);
        self.load(item.id, at, cx);
        cx.notify();
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

    pub(super) fn restore_item_preferences(&mut self) {
        let prefs = self
            .track_prefs
            .playback
            .get(&self.playback.source.id)
            .copied()
            .unwrap_or_default();
        self.speed = prefs.speed.unwrap_or(1.);
        self.audio_delay = 0.;
        self.sync_hud = None;
        self.max_bitrate_mbps = if prefs.bitrate_set {
            prefs.max_bitrate_mbps
        } else {
            self.default_bitrate_mbps
        };
    }

    pub(super) fn set_speed(&mut self, speed: f64) {
        if let Some(mpv) = self.active_mpv()
            && mpv.set_speed(speed).is_ok()
        {
            self.speed = speed;
            self.track_prefs
                .playback
                .entry(self.playback.source.id.clone())
                .or_default()
                .speed = Some(speed);
        }
    }

    pub(super) fn select_track(&mut self, kind: TrackKind, id: Option<i64>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.select_track(kind, id);
        }
        self.remember_track(kind, id);
    }

    /// Not remembered: track memory only covers the primary subtitle.
    pub(super) fn select_secondary_subtitle(&mut self, id: Option<i64>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.select_secondary_subtitle(id);
        }
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

    pub(super) fn toggle_subtitle(&mut self) {
        let tracks: Vec<&Track> = self
            .playback
            .tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Subtitle)
            .collect();
        if let Some(selected) = tracks.iter().find(|t| t.selected) {
            let id = selected.id;
            self.select_track(TrackKind::Subtitle, None);
            if let Some(item) = &self.playback.item
                && let Some(choice) = self.track_prefs.memory.get_mut(&item.id)
            {
                choice.subtitle = Some(id);
            }
        } else {
            let remembered = self
                .playback
                .item
                .as_ref()
                .and_then(|item| self.track_prefs.memory.get(&item.id))
                .and_then(|choice| choice.subtitle);
            let id = remembered
                .filter(|id| tracks.iter().any(|t| t.id == *id))
                .or_else(|| tracks.first().map(|t| t.id));
            if let Some(id) = id {
                self.select_track(TrackKind::Subtitle, Some(id));
            }
        }
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
                crate::log::write(&format!("playback report failed: {err}"));
            }
        }
    })
    .detach();
    tx
}

#[cfg(test)]
mod tests {
    use super::SubtitleReadiness;
    use crate::mpv::{FileEvent, MpvEvent};

    #[test]
    fn selection_waits_until_all_subtitle_additions_settle() {
        let mut readiness = SubtitleReadiness::default();
        assert!(readiness.begin(2));
        assert!(!readiness.take_ready(true, true));
        readiness.settle();
        assert!(!readiness.take_ready(true, true));
        readiness.settle();
        assert!(readiness.take_ready(true, true));
        assert!(!readiness.take_ready(true, true));
    }

    #[test]
    fn failed_additions_settle_and_no_additions_do_not_wait() {
        let mut readiness = SubtitleReadiness::default();
        assert!(readiness.begin(1));
        readiness.settle();
        assert!(readiness.take_ready(true, true));

        readiness = SubtitleReadiness::default();
        assert!(readiness.begin(0));
        assert!(readiness.take_ready(true, false));
    }

    #[test]
    fn track_lists_before_external_subtitles_are_queued_do_not_apply() {
        let mut readiness = SubtitleReadiness::default();
        // embedded tracks land before FileLoaded queues the external ones
        assert!(!readiness.take_ready(true, true));
        assert!(readiness.begin(1));
        // tracks change while the download is still in flight
        assert!(!readiness.take_ready(true, true));
        readiness.settle();
        assert!(readiness.take_ready(true, true));
    }

    #[test]
    fn without_external_subtitles_the_first_track_list_applies() {
        let mut readiness = SubtitleReadiness::default();
        assert!(!readiness.take_ready(false, false));
        assert!(readiness.take_ready(true, false));
    }

    #[test]
    fn a_repeat_file_loaded_queues_nothing_and_keeps_the_count() {
        let mut readiness = SubtitleReadiness::default();
        assert!(readiness.begin(2));
        readiness.settle();
        assert!(!readiness.begin(2));
        assert!(!readiness.take_ready(true, true));
        readiness.settle();
        assert!(readiness.take_ready(true, true));
    }

    #[test]
    fn completions_beyond_the_count_do_not_reapply() {
        let mut readiness = SubtitleReadiness::default();
        assert!(readiness.begin(1));
        readiness.settle();
        assert!(readiness.take_ready(true, true));
        readiness.settle();
        assert!(!readiness.take_ready(true, true));
    }

    #[test]
    fn a_superseded_load_cannot_settle_the_current_one() {
        // Episode switch and bitrate reload start a fresh `Playback` with a new load id;
        // the old load's replies are dropped before they reach its readiness
        let late = MpvEvent::File {
            load: 1,
            event: FileEvent::SubtitleAdded(Ok(())),
        };
        assert!(!late.is_current(Some(2)));
        let failed = MpvEvent::File {
            load: 1,
            event: FileEvent::SubtitleAdded(Err("404".into())),
        };
        assert!(!failed.is_current(Some(2)));
    }
}
