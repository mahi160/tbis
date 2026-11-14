use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub server: String,
    pub user_id: String,
    pub user_name: String,
    pub token: String,
}

/// Order of the Movies and Series pages. Ties fall back to name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Sort {
    Name,
    #[default]
    DateAdded,
    Year,
    Rating,
}

impl Sort {
    pub const ALL: [Sort; 4] = [Sort::Name, Sort::DateAdded, Sort::Year, Sort::Rating];

    pub fn label(self) -> &'static str {
        match self {
            Sort::Name => "Name",
            Sort::DateAdded => "Date added",
            Sort::Year => "Year",
            Sort::Rating => "Rating",
        }
    }

    /// Series "added" means newest Episode, like the Home Series row.
    pub(super) fn query(self, kind: Kind) -> &'static str {
        match (self, kind) {
            (Sort::Name, _) => "SortBy=SortName&SortOrder=Ascending",
            (Sort::DateAdded, Kind::Series) => {
                "SortBy=DateLastContentAdded,SortName&SortOrder=Descending,Ascending"
            }
            (Sort::DateAdded, _) => "SortBy=DateCreated,SortName&SortOrder=Descending,Ascending",
            (Sort::Year, _) => "SortBy=ProductionYear,SortName&SortOrder=Descending,Ascending",
            (Sort::Rating, _) => "SortBy=CommunityRating,SortName&SortOrder=Descending,Ascending",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Item {
    pub id: String,
    /// Series of an Episode.
    pub series_id: Option<String>,
    /// Season of an Episode.
    pub season_id: Option<String>,
    pub name: String,
    pub production_year: Option<i32>,
    #[serde(default)]
    pub(super) image_tags: HashMap<String, String>,
    #[serde(default)]
    pub(super) backdrop_image_tags: Vec<String>,
    /// Series art for Episodes.
    pub(super) parent_thumb_item_id: Option<String>,
    pub(super) parent_thumb_image_tag: Option<String>,
    pub(super) parent_backdrop_item_id: Option<String>,
    #[serde(default)]
    pub(super) parent_backdrop_image_tags: Vec<String>,
    #[serde(default)]
    pub user_data: UserData,
    /// Episodes only.
    pub series_name: Option<String>,
    /// Season number (Episodes only).
    pub parent_index_number: Option<i32>,
    /// Episode number, or season number for a season.
    pub index_number: Option<i32>,
    pub run_time_ticks: Option<i64>,
    /// ISO 8601 air/release date, e.g. `2024-05-01T00:00:00.0000000Z`.
    pub premiere_date: Option<String>,
    pub overview: Option<String>,
    /// Detail-page metadata; single-item requests carry them, list requests mostly don't.
    pub community_rating: Option<f32>,
    /// Critics score, 0-100.
    pub critic_rating: Option<f32>,
    /// Age rating, e.g. `PG-13`, `TV-MA`.
    pub official_rating: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub people: Vec<Person>,
    /// Streams of the default media source; single-item requests only.
    #[serde(default)]
    media_streams: Vec<StreamInfo>,
    /// unverified: assumes the server always sends `Type`; falls back to `Kind::Other`
    /// when it's missing or not one of the kinds this app knows about (e.g. Season).
    #[serde(rename = "Type", default)]
    pub kind: Kind,
}

impl Item {
    /// What a click on this item's title opens: an Episode's Series, else itself.
    /// The stub only needs id/name/kind; Series detail loads the rest.
    pub fn detail_target(&self) -> Item {
        match (&self.kind, &self.series_id) {
            (Kind::Episode, Some(series_id)) => Item {
                id: series_id.clone(),
                name: self.series_name.clone().unwrap_or_default(),
                kind: Kind::Series,
                ..Item::default()
            },
            _ => self.clone(),
        }
    }

    /// Special format tags (4K, HDR kind, premium audio, surround); see `media_tags`.
    pub fn media_tags(&self) -> Vec<&'static str> {
        media_tags(&self.media_streams)
    }

    /// `S02E03 · Name`, or just the name when numbers are missing.
    pub fn episode_label(&self) -> String {
        match (self.parent_index_number, self.index_number) {
            (Some(season), Some(episode)) => {
                format!(
                    "{} · {}",
                    episode_code(season, episode),
                    self.display_name()
                )
            }
            _ => self.name.clone(),
        }
    }

    /// Name without a leading "Anything - S04E15 - " matching this Episode's own numbers.
    pub fn display_name(&self) -> &str {
        let (Some(season), Some(episode)) = (self.parent_index_number, self.index_number) else {
            return &self.name;
        };
        for (at, sep) in self.name.match_indices(" - ") {
            let rest = &self.name[at + sep.len()..];
            if let Some((code, title)) = rest.split_once(" - ")
                && !title.trim().is_empty()
                && code_matches(code, season, episode)
            {
                return title;
            }
        }
        &self.name
    }
}

/// Format facts of one stream, read for the detail page's media tags.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StreamInfo {
    #[serde(rename = "Type")]
    kind: String,
    codec: Option<String>,
    profile: Option<String>,
    display_title: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    channels: Option<i32>,
    video_range: Option<String>,
    video_range_type: Option<String>,
    #[serde(default)]
    is_default: bool,
}

/// Tags only for formats worth calling out, best first: 4K, HDR kind, premium audio,
/// surround layout (e.g. `4K`, `Dolby Vision`, `Dolby Atmos`, `7.1`). Plain HD/SD,
/// SDR, stereo, and ordinary codecs get none.
fn media_tags(streams: &[StreamInfo]) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if let Some(video) = streams.iter().find(|s| s.kind == "Video") {
        let (w, h) = (video.width.unwrap_or(0), video.height.unwrap_or(0));
        if w >= 3200 || h >= 2000 {
            tags.push("4K");
        }
        let range = video.video_range_type.as_deref().unwrap_or_default();
        tags.extend(match range {
            _ if range.starts_with("DOVI") => Some("Dolby Vision"),
            "HDR10Plus" => Some("HDR10+"),
            "HDR10" => Some("HDR10"),
            "HLG" => Some("HLG"),
            _ if video.video_range.as_deref() == Some("HDR") => Some("HDR"),
            _ => None,
        });
    }
    let mut audio_streams = streams.iter().filter(|s| s.kind == "Audio");
    let audio = audio_streams
        .clone()
        .find(|s| s.is_default)
        .or_else(|| audio_streams.next());
    if let Some(audio) = audio {
        let text =
            |field: &Option<String>| field.as_deref().unwrap_or_default().to_ascii_lowercase();
        let (profile, title) = (text(&audio.profile), text(&audio.display_title));
        let atmos = profile.contains("atmos") || title.contains("atmos");
        tags.extend(match text(&audio.codec).as_str() {
            _ if atmos => Some("Dolby Atmos"),
            "truehd" => Some("Dolby TrueHD"),
            "eac3" => Some("Dolby Digital+"),
            "ac3" => Some("Dolby Digital"),
            "dts" if profile.contains("dts:x") || title.contains("dts:x") => Some("DTS:X"),
            "dts" if profile.contains("ma") => Some("DTS-HD MA"),
            "dts" => Some("DTS"),
            _ => None,
        });
        tags.extend(match audio.channels {
            Some(8) => Some("7.1"),
            Some(6) => Some("5.1"),
            _ => None,
        });
    }
    tags
}

/// Cast or crew member of an item.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Person {
    pub id: String,
    pub name: String,
    /// Character played, for actors.
    pub role: Option<String>,
    /// `Actor`, `Director`, `Writer`, ...
    #[serde(rename = "Type")]
    pub kind: Option<String>,
    pub(super) primary_image_tag: Option<String>,
}

/// `S04E15`, zero-padded; the one format used everywhere an Episode's numbers show.
pub fn episode_code(season: i32, episode: i32) -> String {
    format!("S{season:02}E{episode:02}")
}

/// `S04E15` or `S04E18-19` (any case, any zero padding) naming this season and first episode.
fn code_matches(code: &str, season: i32, episode: i32) -> bool {
    let number = |s: &str| {
        (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse::<i32>().ok())
            .flatten()
    };
    let code = code.to_ascii_uppercase();
    let Some((s, e)) = code.strip_prefix('S').and_then(|c| c.split_once('E')) else {
        return false;
    };
    let (first, last) = e.split_once('-').unwrap_or((e, e));
    number(s) == Some(season) && number(first) == Some(episode) && number(last).is_some()
}

/// Item type searched on its own, so one type can't crowd out the others; also
/// `Item::kind`, read off the server's own `Type` field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Kind {
    Movie,
    Series,
    Episode,
    /// Season and anything else this app doesn't render its own way.
    #[serde(other)]
    #[default]
    Other,
}

impl Kind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Kind::Movie => "Movie",
            Kind::Series => "Series",
            Kind::Episode => "Episode",
            Kind::Other => "Item",
        }
    }
}

/// A Jellyfin media folder that holds Movies or Series.
#[allow(dead_code)] // used by the Modern view (roadmap 31)
#[derive(Clone, Debug, PartialEq)]
pub struct Library {
    pub id: String,
    pub name: String,
    pub kind: Kind,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserData {
    #[serde(default)]
    pub played: bool,
    #[serde(default)]
    pub playback_position_ticks: i64,
    /// Only present while partly watched.
    pub played_percentage: Option<f64>,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub play_count: u32,
    /// ISO 8601, e.g. `2024-05-01T20:15:00.0000000Z`.
    pub last_played_date: Option<String>,
}

/// Fresh per-play details: which file to stream and where to resume.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaybackItem {
    pub id: String,
    /// Episodes only; used to find the next Episode.
    pub series_id: Option<String>,
    #[serde(default)]
    pub(super) media_sources: Vec<MediaSource>,
    #[serde(default)]
    pub user_data: UserData,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    trickplay: HashMap<String, HashMap<String, TrickplayInfo>>,
    #[serde(default)]
    media_streams: Vec<StreamInfo>,
    /// How this play streams; filled in by `Api::playback_item`.
    #[serde(skip)]
    pub stream: Stream,
}

/// URL mpv plays, and whether it is a server transcode.
#[derive(Clone, Debug, Default)]
pub struct Stream {
    pub url: String,
    /// Server's PlaySessionId of a transcode; `None` for direct play.
    pub transcode: Option<String>,
}

impl PlaybackItem {
    /// Special format tags shown beside the Player's clock; see `media_tags`.
    pub fn media_tags(&self) -> Vec<&'static str> {
        media_tags(&self.media_streams)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct MediaSource {
    pub(super) id: String,
    #[serde(default)]
    pub(super) media_streams: Vec<MediaStream>,
}

/// One audio/video/subtitle stream of a `MediaSource`; only subtitle fields are read.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct MediaStream {
    #[serde(rename = "Type")]
    pub(super) kind: String,
    pub(super) index: i64,
    #[serde(default)]
    pub(super) is_external: bool,
    #[serde(default)]
    pub(super) is_default: bool,
    pub(super) display_title: Option<String>,
    pub(super) language: Option<String>,
}

/// A chapter marker; Jellyfin's own (possibly auto-generated), not mpv's embedded ones --
/// present on more files and carries a name.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Chapter {
    pub name: Option<String>,
    start_position_ticks: i64,
}

/// Server-detected range the Player offers to skip (Jellyfin 10.10+ media segments).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub kind: SegmentKind,
    pub start: f64,
    pub end: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SegmentKind {
    Intro,
    /// Credits.
    Outro,
}

impl Chapter {
    pub fn start_seconds(&self) -> f64 {
        ticks_to_seconds(self.start_position_ticks)
    }
}

/// One sprite-sheet resolution of scrub-preview tiles for a `MediaSource`.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TrickplayInfo {
    pub width: i32,
    pub height: i32,
    pub tile_width: i32,
    pub tile_height: i32,
    pub thumbnail_count: i32,
    pub interval: i32,
}

impl TrickplayInfo {
    /// Which sprite tile holds the thumbnail closest to `seconds`, and that
    /// thumbnail's pixel offset within the tile.
    pub fn thumbnail_at(&self, seconds: f64) -> (i32, i32, i32) {
        let tile_width = self.tile_width.max(1);
        let per_tile = (tile_width * self.tile_height).max(1);
        let index = ((seconds * 1000. / self.interval.max(1) as f64) as i32)
            .clamp(0, (self.thumbnail_count - 1).max(0));
        let in_tile = index % per_tile;
        let (col, row) = (in_tile % tile_width, in_tile / tile_width);
        (index / per_tile, col * self.width, row * self.height)
    }
}

/// An external (sidecar) text subtitle: not muxed into the stream mpv loads, so it
/// needs its own `sub-add` once the file is open.
pub struct ExternalSubtitle {
    pub url: String,
    pub title: String,
    pub lang: String,
    pub is_default: bool,
}

impl PlaybackItem {
    pub fn media_source_id(&self) -> &str {
        self.media_sources.first().map_or(&self.id, |m| &m.id)
    }

    pub fn resume_seconds(&self) -> f64 {
        ticks_to_seconds(self.user_data.playback_position_ticks)
    }

    /// Widest available trickplay sprite sheet for the media source mpv is playing.
    pub(super) fn trickplay(&self) -> Option<TrickplayInfo> {
        self.trickplay
            .get(self.media_source_id())?
            .values()
            .max_by_key(|t| t.width)
            .copied()
    }
}

pub enum Report {
    Start,
    Progress,
    Stopped,
}

const TICKS_PER_SECOND: f64 = 10_000_000.0;

pub(super) fn ticks_to_seconds(ticks: i64) -> f64 {
    ticks as f64 / TICKS_PER_SECOND
}

pub(super) fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * TICKS_PER_SECOND) as i64
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PublicUser {
    pub name: String,
    #[serde(default)]
    pub has_password: bool,
}

#[cfg(test)]
mod tests {
    use super::Item;

    fn episode(name: &str, season: i32, number: i32) -> Item {
        serde_json::from_value(serde_json::json!({
            "Id": "x", "Name": name, "ParentIndexNumber": season, "IndexNumber": number
        }))
        .unwrap()
    }

    #[::core::prelude::v1::test]
    fn strips_matching_episode_code() {
        let label = |name, s, e| episode(name, s, e).episode_label();
        assert_eq!(
            label("The Office (US) - S04E15 - Night Out", 4, 15),
            "S04E15 · Night Out"
        );
        assert_eq!(
            label("The Office (US) - S04E18-19 - Goodbye, Toby", 4, 18),
            "S04E18 · Goodbye, Toby"
        );
        assert_eq!(
            label("A - B - s4e15 - Title - Part 2", 4, 15),
            "S04E15 · Title - Part 2"
        );
        // code for another episode, or no code: untouched
        assert_eq!(
            label("Show - S04E16 - Night Out", 4, 15),
            "S04E15 · Show - S04E16 - Night Out"
        );
        assert_eq!(
            label("Before - After - End", 1, 1),
            "S01E01 · Before - After - End"
        );
        assert_eq!(
            label("Show - S04E15 - ", 4, 15),
            "S04E15 · Show - S04E15 - "
        );
        assert_eq!(label("Pilot", 1, 1), "S01E01 · Pilot");
    }

    #[test]
    fn media_tags_from_streams() {
        let item: Item = serde_json::from_value(serde_json::json!({
            "Id": "m", "Name": "Movie",
            "MediaStreams": [
                {"Type": "Video", "Width": 3840, "Height": 1600, "VideoRange": "HDR", "VideoRangeType": "DOVIWithHDR10"},
                {"Type": "Audio", "Codec": "eac3", "Channels": 6},
                {"Type": "Audio", "Codec": "truehd", "Profile": "TrueHD + Dolby Atmos", "Channels": 8, "IsDefault": true},
            ]
        }))
        .unwrap();
        assert_eq!(
            item.media_tags(),
            ["4K", "Dolby Vision", "Dolby Atmos", "7.1"]
        );

        let plain: Item = serde_json::from_value(serde_json::json!({
            "Id": "e", "Name": "Ep",
            "MediaStreams": [{"Type": "Video", "Width": 1920, "Height": 1080}, {"Type": "Audio", "Codec": "aac", "Channels": 2}]
        }))
        .unwrap();
        assert!(plain.media_tags().is_empty(), "1080p stereo is not special");
    }
}
