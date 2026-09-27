use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::AsyncReadExt as _;
use futures::channel::mpsc::UnboundedSender;
use gpui_kit::http_client::{AsyncBody, HttpClient, Method, Request, Response, Url};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub server: String,
    pub user_id: String,
    pub user_name: String,
    pub token: String,
}

/// Adds `http://` when no scheme is given and trims trailing slashes.
pub fn normalize_server(input: &str) -> String {
    let s = input.trim().trim_end_matches('/');
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        s.to_string()
    } else {
        format!("http://{s}")
    }
}

fn auth_header(device_id: &str, token: Option<&str>) -> String {
    let mut header = format!(
        r#"MediaBrowser Client="tbis", Device="Mac", DeviceId="{device_id}", Version="{}""#,
        env!("CARGO_PKG_VERSION")
    );
    if let Some(token) = token {
        header.push_str(&format!(r#", Token="{token}""#));
    }
    header
}

async fn read_json<T: DeserializeOwned>(response: Response<AsyncBody>) -> Result<T> {
    let mut bytes = Vec::new();
    response.into_body().read_to_end(&mut bytes).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Sends a request and checks for a success status, shared by every request path
/// that reports errors as `anyhow::Error` (login's is user-facing text instead).
async fn send(
    http: &Arc<dyn HttpClient>,
    request: Request<AsyncBody>,
) -> Result<Response<AsyncBody>> {
    let response = http.send(request).await.map_err(|err| {
        // reqwest's message embeds the full URL (server, user id, token-bearing query); keep only the cause
        let cause = err
            .chain()
            .map(|cause| cause.to_string())
            .filter(|cause| !cause.contains("://"))
            .last()
            .map(|cause| format!(": {cause}"))
            .unwrap_or_default();
        anyhow!("Can't reach the server{cause}")
    })?;
    if !response.status().is_success() {
        return Err(HttpStatus(response.status().as_u16()).into());
    }
    Ok(response)
}

/// Non-success status from the server.
#[derive(Debug)]
struct HttpStatus(u16);

impl std::fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "HTTP {}", self.0)
    }
}

impl std::error::Error for HttpStatus {}

/// Server rejected the saved token (HTTP 401); `Api` also signals its `expired` channel.
#[derive(Debug)]
pub struct SessionExpired;

impl std::fmt::Display for SessionExpired {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Your session expired. Sign in again.")
    }
}

impl std::error::Error for SessionExpired {}

/// Order of the Movies and Series pages. Ties fall back to name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Sort {
    #[default]
    Name,
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
    fn query(self, kind: Kind) -> &'static str {
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
    image_tags: HashMap<String, String>,
    #[serde(default)]
    backdrop_image_tags: Vec<String>,
    /// Series art for Episodes.
    parent_thumb_item_id: Option<String>,
    parent_thumb_image_tag: Option<String>,
    parent_backdrop_item_id: Option<String>,
    #[serde(default)]
    parent_backdrop_image_tags: Vec<String>,
    #[serde(default)]
    pub user_data: UserData,
    /// Episodes only.
    pub series_name: Option<String>,
    /// Season number (Episodes only).
    pub parent_index_number: Option<i32>,
    /// Episode number, or season number for a season.
    pub index_number: Option<i32>,
    pub run_time_ticks: Option<i64>,
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
    primary_image_tag: Option<String>,
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

/// Tags needed by `Api::wide_image_url`.
const WIDE_IMAGES: &str = "EnableImageTypes=Primary,Thumb,Backdrop&ImageTypeLimit=1";

/// Tags needed by `Api::poster_url`, shared by every plain item listing.
const POSTER_IMAGES: &str = "EnableImageTypes=Primary&ImageTypeLimit=1";

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
    fn as_str(self) -> &'static str {
        match self {
            Kind::Movie => "Movie",
            Kind::Series => "Series",
            Kind::Episode => "Episode",
            Kind::Other => "Item",
        }
    }
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
}

/// Fresh per-play details: which file to stream and where to resume.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaybackItem {
    pub id: String,
    /// Episodes only; used to find the next Episode.
    pub series_id: Option<String>,
    #[serde(default)]
    media_sources: Vec<MediaSource>,
    #[serde(default)]
    pub user_data: UserData,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    trickplay: HashMap<String, HashMap<String, TrickplayInfo>>,
    #[serde(default)]
    media_streams: Vec<StreamInfo>,
}

impl PlaybackItem {
    /// Special format tags shown beside the Player's clock; see `media_tags`.
    pub fn media_tags(&self) -> Vec<&'static str> {
        media_tags(&self.media_streams)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MediaSource {
    id: String,
    #[serde(default)]
    media_streams: Vec<MediaStream>,
}

/// One audio/video/subtitle stream of a `MediaSource`; only subtitle fields are read.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MediaStream {
    #[serde(rename = "Type")]
    kind: String,
    #[serde(default)]
    is_external: bool,
    #[serde(default)]
    is_default: bool,
    delivery_url: Option<String>,
    display_title: Option<String>,
    language: Option<String>,
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

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SegmentDto {
    #[serde(rename = "Type")]
    kind: String,
    start_ticks: i64,
    end_ticks: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SegmentsResult {
    items: Vec<SegmentDto>,
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
    fn trickplay(&self) -> Option<TrickplayInfo> {
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

fn ticks_to_seconds(ticks: i64) -> f64 {
    ticks as f64 / TICKS_PER_SECOND
}

fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * TICKS_PER_SECOND) as i64
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemsResult {
    items: Vec<Item>,
}

/// Signed-in client; every request carries the session token.
#[derive(Clone)]
pub struct Api {
    http: Arc<dyn HttpClient>,
    session: Session,
    device_id: String,
    /// Fired on every 401 so the app can drop the session and return to sign-in.
    expired: UnboundedSender<()>,
}

impl Api {
    pub fn new(
        http: Arc<dyn HttpClient>,
        session: Session,
        device_id: String,
        expired: UnboundedSender<()>,
    ) -> Self {
        Self {
            http,
            session,
            device_id,
            expired,
        }
    }

    async fn send(&self, request: Request<AsyncBody>) -> Result<Response<AsyncBody>> {
        send(&self.http, request).await.map_err(|err| {
            if let Some(HttpStatus(401)) = err.downcast_ref() {
                // receiver gone means app already left Main; nothing to signal
                let _ = self.expired.unbounded_send(());
                SessionExpired.into()
            } else {
                err
            }
        })
    }

    async fn get<T: DeserializeOwned>(&self, path_and_query: &str) -> Result<T> {
        let request = Request::builder()
            .uri(format!("{}{path_and_query}", self.session.server))
            .header("Authorization", self.auth_header())
            .body(AsyncBody::empty())?;
        read_json(self.send(request).await?).await
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> Result<()> {
        self.request(Method::POST, path, body).await
    }

    async fn request(&self, method: Method, path: &str, body: serde_json::Value) -> Result<()> {
        let request = Request::builder()
            .method(method)
            .uri(format!("{}{path}", self.session.server))
            .header("Content-Type", "application/json")
            .header("Authorization", self.auth_header())
            .body(AsyncBody::from(body.to_string()))?;
        self.send(request).await?;
        Ok(())
    }

    /// `Items?...&{extra}`, with the `Fields`/image params every item listing shares.
    async fn items(&self, kind: Kind, extra: &str) -> Result<Vec<Item>> {
        let query = format!(
            "/Items?userId={}&IncludeItemTypes={}&Recursive=true&Fields=ProductionYear&{POSTER_IMAGES}&{extra}",
            self.session.user_id,
            kind.as_str(),
        );
        Ok(self.get::<ItemsResult>(&query).await?.items)
    }

    pub fn auth_header(&self) -> String {
        auth_header(&self.device_id, Some(&self.session.token))
    }

    pub async fn playback_item(&self, item_id: &str) -> Result<PlaybackItem> {
        self.get(&format!(
            "/Items/{item_id}?userId={}&Fields=Chapters,Trickplay",
            self.session.user_id
        ))
        .await
    }

    /// Intro and credits ranges of one item, by start time.
    pub async fn segments(&self, item_id: &str) -> Result<Vec<Segment>> {
        let path =
            format!("/MediaSegments/{item_id}?includeSegmentTypes=Intro&includeSegmentTypes=Outro");
        let mut segments: Vec<Segment> = self
            .get::<SegmentsResult>(&path)
            .await?
            .items
            .into_iter()
            .filter_map(|dto| {
                let kind = match dto.kind.as_str() {
                    "Intro" => SegmentKind::Intro,
                    "Outro" => SegmentKind::Outro,
                    _ => return None,
                };
                Some(Segment {
                    kind,
                    start: ticks_to_seconds(dto.start_ticks),
                    end: ticks_to_seconds(dto.end_ticks),
                })
            })
            .filter(|segment| segment.end > segment.start)
            .collect();
        segments.sort_by(|a, b| a.start.total_cmp(&b.start));
        Ok(segments)
    }

    /// Original file, never transcoded.
    pub fn stream_url(&self, item: &PlaybackItem) -> String {
        format!(
            "{}/Videos/{}/stream?static=true&mediaSourceId={}",
            self.session.server,
            item.id,
            item.media_source_id()
        )
    }

    /// Sidecar text subtitles mpv can't see in the stream mpv opens (not muxed into
    /// the container), each needing its own `sub-add` once that file is loaded.
    pub fn external_subtitles(&self, item: &PlaybackItem) -> Vec<ExternalSubtitle> {
        let Some(source) = item
            .media_sources
            .iter()
            .find(|m| m.id == item.media_source_id())
        else {
            return Vec::new();
        };
        source
            .media_streams
            .iter()
            .filter(|s| s.kind == "Subtitle" && s.is_external)
            .filter_map(|s| {
                let url = s.delivery_url.as_ref()?;
                Some(ExternalSubtitle {
                    url: format!("{}{url}", self.session.server),
                    title: s.display_title.clone().unwrap_or_default(),
                    lang: s.language.clone().unwrap_or_default(),
                    is_default: s.is_default,
                })
            })
            .collect()
    }

    /// Widest scrub-preview sprite sheet the server generated for this item, if any.
    pub fn trickplay(&self, item: &PlaybackItem) -> Option<TrickplayInfo> {
        item.trickplay()
    }

    /// One sprite-sheet tile (a grid of `info.tile_width`x`info.tile_height` thumbnails).
    pub fn trickplay_tile_url(
        &self,
        item: &PlaybackItem,
        info: &TrickplayInfo,
        tile: i32,
    ) -> String {
        format!(
            "{}/Videos/{}/Trickplay/{}/{tile}.jpg?mediaSourceId={}",
            self.session.server,
            item.id,
            info.width,
            item.media_source_id()
        )
    }

    pub async fn report(
        &self,
        report: Report,
        item: &PlaybackItem,
        play_session_id: &str,
        seconds: f64,
        paused: bool,
    ) -> Result<()> {
        let path = match report {
            Report::Start => "/Sessions/Playing",
            Report::Progress => "/Sessions/Playing/Progress",
            Report::Stopped => "/Sessions/Playing/Stopped",
        };
        let body = serde_json::json!({
            "ItemId": item.id,
            "MediaSourceId": item.media_source_id(),
            "PlaySessionId": play_session_id,
            "PositionTicks": seconds_to_ticks(seconds),
            "IsPaused": paused,
            "PlayMethod": "DirectPlay",
            "CanSeek": true,
        });
        self.post(path, body).await
    }

    /// Marks played (a Series: every Episode) or unplayed; either way the server
    /// resets the saved resume position.
    pub async fn set_played(&self, item_id: &str, played: bool) -> Result<()> {
        let path = format!("/Users/{}/PlayedItems/{item_id}", self.session.user_id);
        let method = if played { Method::POST } else { Method::DELETE };
        self.request(method, &path, serde_json::Value::Null).await
    }

    pub async fn set_favorite(&self, item_id: &str, favorite: bool) -> Result<()> {
        let path = format!("/Users/{}/FavoriteItems/{item_id}", self.session.user_id);
        let method = if favorite {
            Method::POST
        } else {
            Method::DELETE
        };
        self.request(method, &path, serde_json::Value::Null).await
    }

    /// Every item of one kind (Movie or Series) from all Libraries.
    pub async fn library(&self, kind: Kind, sort: Sort) -> Result<Vec<Item>> {
        self.items(kind, sort.query(kind)).await
    }

    /// Any item with its overview.
    pub async fn item(&self, id: &str) -> Result<Item> {
        self.get(&format!("/Items/{id}?userId={}", self.session.user_id))
            .await
    }

    /// Numbered seasons first, Specials (season 0) last.
    pub async fn seasons(&self, series_id: &str) -> Result<Vec<Item>> {
        let path = format!("/Shows/{series_id}/Seasons?userId={}", self.session.user_id);
        let mut seasons = self.get::<ItemsResult>(&path).await?.items;
        seasons.sort_by_key(|s| match s.index_number {
            Some(0) | None => i32::MAX,
            Some(n) => n,
        });
        Ok(seasons)
    }

    pub async fn episodes(&self, series_id: &str, season_id: &str) -> Result<Vec<Item>> {
        let path = format!(
            "/Shows/{series_id}/Episodes?userId={}&seasonId={season_id}&EnableImageTypes=Primary&ImageTypeLimit=1",
            self.session.user_id
        );
        Ok(self.get::<ItemsResult>(&path).await?.items)
    }

    /// Episode after `episode_id` in the server's order, crossing seasons.
    pub async fn next_episode(&self, series_id: &str, episode_id: &str) -> Result<Option<Item>> {
        let path = format!(
            "/Shows/{series_id}/Episodes?userId={}&startItemId={episode_id}&Limit=2",
            self.session.user_id
        );
        let episodes = self.get::<ItemsResult>(&path).await?.items;
        Ok(episodes.into_iter().find(|e| e.id != episode_id))
    }

    /// Next unwatched Episode of one Series, if any.
    pub async fn next_up(&self, series_id: &str) -> Result<Option<Item>> {
        let path = format!(
            "/Shows/NextUp?seriesId={series_id}&userId={}&Limit=1",
            self.session.user_id
        );
        Ok(self
            .get::<ItemsResult>(&path)
            .await?
            .items
            .into_iter()
            .next())
    }

    /// Started, unfinished Movies and Episodes, most recent first.
    pub async fn resume(&self, limit: usize) -> Result<Vec<Item>> {
        let path = format!(
            "/UserItems/Resume?userId={}&IncludeItemTypes=Movie,Episode&Limit={limit}&{WIDE_IMAGES}",
            self.session.user_id
        );
        Ok(self.get::<ItemsResult>(&path).await?.items)
    }

    /// Next Episode of every followed Series.
    pub async fn next_up_all(&self, limit: usize) -> Result<Vec<Item>> {
        let path = format!(
            "/Shows/NextUp?userId={}&Limit={limit}&{WIDE_IMAGES}",
            self.session.user_id
        );
        Ok(self.get::<ItemsResult>(&path).await?.items)
    }

    /// Unplayed items of one kind, newest first (Series by newest Episode).
    pub async fn latest_unplayed(&self, kind: Kind, limit: usize) -> Result<Vec<Item>> {
        self.items(
            kind,
            &format!(
                "isPlayed=false&Limit={limit}&{}",
                Sort::DateAdded.query(kind)
            ),
        )
        .await
    }

    /// Server-side search across all Libraries, one type at a time.
    pub async fn search(&self, term: &str, kind: Kind, limit: usize) -> Result<Vec<Item>> {
        let term: String = url::form_urlencoded::byte_serialize(term.as_bytes()).collect();
        self.items(kind, &format!("searchTerm={term}&Limit={limit}"))
            .await
    }

    /// 16:9 art. Episode: still, Series thumb, Series backdrop. Movie: thumb, backdrop, poster.
    pub fn wide_image_url(&self, item: &Item) -> Option<String> {
        let own = |kind: &'static str, tag: Option<&String>| {
            tag.map(|t| (item.id.clone(), kind, t.clone()))
        };
        let parent = |kind: &'static str, id: &Option<String>, tag: Option<&String>| {
            id.clone().zip(tag.cloned()).map(|(id, t)| (id, kind, t))
        };
        let pick = if item.kind == Kind::Episode {
            own("Primary", item.image_tags.get("Primary"))
                .or_else(|| {
                    parent(
                        "Thumb",
                        &item.parent_thumb_item_id,
                        item.parent_thumb_image_tag.as_ref(),
                    )
                })
                .or_else(|| {
                    parent(
                        "Backdrop/0",
                        &item.parent_backdrop_item_id,
                        item.parent_backdrop_image_tags.first(),
                    )
                })
        } else {
            own("Thumb", item.image_tags.get("Thumb"))
                .or_else(|| own("Backdrop/0", item.backdrop_image_tags.first()))
                .or_else(|| own("Primary", item.image_tags.get("Primary")))
        };
        let (id, kind, tag) = pick?;
        Some(format!(
            "{}/Items/{id}/Images/{kind}?fillWidth=480&quality=90&tag={tag}",
            self.session.server
        ))
    }

    /// Raw bytes of an image URL from this API (e.g. Now Playing artwork).
    pub async fn image_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let request = Request::builder()
            .uri(url)
            .header("Authorization", self.auth_header())
            .body(AsyncBody::empty())?;
        let mut bytes = Vec::new();
        self.send(request)
            .await?
            .into_body()
            .read_to_end(&mut bytes)
            .await?;
        Ok(bytes)
    }

    /// Full-width backdrop art for detail pages.
    pub fn backdrop_url(&self, item: &Item) -> Option<String> {
        let tag = item.backdrop_image_tags.first()?;
        Some(format!(
            "{}/Items/{}/Images/Backdrop/0?fillWidth=1920&quality=85&tag={tag}",
            self.session.server, item.id
        ))
    }

    pub fn person_image_url(&self, person: &Person) -> Option<String> {
        let tag = person.primary_image_tag.as_ref()?;
        Some(format!(
            "{}/Items/{}/Images/Primary?fillWidth=128&quality=90&tag={tag}",
            self.session.server, person.id
        ))
    }

    pub fn poster_url(&self, item: &Item) -> Option<String> {
        let tag = item.image_tags.get("Primary")?;
        Some(format!(
            "{}/Items/{}/Images/Primary?fillWidth=320&quality=90&tag={tag}",
            self.session.server, item.id
        ))
    }
}

#[derive(Deserialize)]
struct PublicSystemInfo {
    #[serde(rename = "Version")]
    _version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AuthResult {
    access_token: String,
    user: AuthUser,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AuthUser {
    id: String,
    name: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PublicUser {
    pub name: String,
    #[serde(default)]
    pub has_password: bool,
}

/// Users the server lists on its sign-in screen; empty when it hides them.
pub async fn public_users(
    http: Arc<dyn HttpClient>,
    server_input: &str,
) -> Result<Vec<PublicUser>> {
    let server = normalize_server(server_input);
    Url::parse(&server)?;
    let request = Request::builder()
        .uri(format!("{server}/Users/Public"))
        .body(AsyncBody::empty())?;
    read_json(send(&http, request).await?).await
}

/// Checks the server is Jellyfin, then signs in. Errors are user-facing messages.
pub async fn login(
    http: Arc<dyn HttpClient>,
    server_input: &str,
    username: &str,
    password: &str,
    device_id: &str,
) -> Result<Session, String> {
    if server_input.trim().is_empty() {
        return Err("Enter a server URL.".into());
    }
    if username.trim().is_empty() {
        return Err("Enter a username.".into());
    }
    let server = normalize_server(server_input);
    if Url::parse(&server).is_err() {
        return Err(format!("Invalid server URL: {server}"));
    }

    let unreachable = |_| format!("Cannot reach server at {server}.");
    let not_jellyfin = || format!("No Jellyfin server found at {server}.");

    let request = Request::builder()
        .uri(format!("{server}/System/Info/Public"))
        .body(AsyncBody::empty())
        .map_err(|_| not_jellyfin())?;
    let response = http.send(request).await.map_err(unreachable)?;
    if !response.status().is_success() {
        return Err(not_jellyfin());
    }
    read_json::<PublicSystemInfo>(response)
        .await
        .map_err(|_| not_jellyfin())?;

    let body = serde_json::json!({ "Username": username.trim(), "Pw": password }).to_string();
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("{server}/Users/AuthenticateByName"))
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header(device_id, None))
        .body(AsyncBody::from(body))
        .map_err(|_| not_jellyfin())?;
    let response = http.send(request).await.map_err(unreachable)?;
    let status = response.status();
    if status.as_u16() == 401 {
        return Err("Incorrect username or password.".into());
    }
    if !status.is_success() {
        return Err(format!("Sign in failed (HTTP {}).", status.as_u16()));
    }
    let auth: AuthResult = read_json(response)
        .await
        .map_err(|_| "Sign in failed: unexpected response from server.".to_string())?;

    Ok(Session {
        server,
        user_id: auth.user.id,
        user_name: auth.user.name,
        token: auth.access_token,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Api, Item, Segment, SegmentKind, Session, SessionExpired, login, normalize_server,
    };
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::sync::Arc;

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

    /// Minimal fake Jellyfin: accepts password "right", or empty for passwordless "guest".
    fn fake_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let (status, body) = if req.starts_with("GET /System/Info/Public") {
                    ("200 OK", r#"{"Version":"10.10.0"}"#)
                } else if req.starts_with("POST /Users/AuthenticateByName") {
                    let mahi =
                        req.contains(r#""Username":"mahi""#) && req.contains(r#""Pw":"right""#);
                    let guest = req.contains(r#""Username":"guest""#) && req.contains(r#""Pw":"""#);
                    if (mahi || guest) && req.contains("DeviceId=\"dev\"") {
                        (
                            "200 OK",
                            r#"{"AccessToken":"tok","User":{"Id":"u1","Name":"mahi"}}"#,
                        )
                    } else {
                        ("401 Unauthorized", "")
                    }
                } else if req.starts_with("GET /MediaSegments/ep1") {
                    (
                        "200 OK",
                        r#"{"Items":[{"Type":"Outro","StartTicks":12000000000,"EndTicks":13000000000},{"Type":"Recap","StartTicks":0,"EndTicks":100},{"Type":"Intro","StartTicks":300000000,"EndTicks":900000000}]}"#,
                    )
                } else if req.starts_with("GET /Expired") {
                    ("401 Unauthorized", "")
                } else {
                    ("404 Not Found", "")
                };
                let resp = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(resp.as_bytes()).unwrap();
            }
        });
        format!("{addr}/")
    }

    #[test]
    fn login_paths() {
        let http = Arc::new(reqwest_client::ReqwestClient::new());
        let server = fake_server();
        let run_as = |server: &str, user: &str, pw: &str| {
            futures::executor::block_on(login(http.clone(), server, user, pw, "dev"))
        };
        let run = |server: &str, pw: &str| run_as(server, "mahi", pw);

        let session = run(&server, "right").unwrap();
        assert_eq!(
            session.server,
            format!("http://{}", server.trim_end_matches('/'))
        );
        assert_eq!(
            (
                session.user_id.as_str(),
                session.user_name.as_str(),
                session.token.as_str()
            ),
            ("u1", "mahi", "tok")
        );

        assert!(
            run_as(&server, "guest", "").is_ok(),
            "passwordless user signs in"
        );
        assert_eq!(
            run(&server, "wrong").unwrap_err(),
            "Incorrect username or password."
        );
        assert!(
            run("127.0.0.1:1", "right")
                .unwrap_err()
                .starts_with("Cannot reach server")
        );
        assert!(
            run("", "right")
                .unwrap_err()
                .starts_with("Enter a server URL")
        );
    }

    #[test]
    fn unauthorized_signals_session_expired() {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let session = Session {
            server: format!("http://{}", fake_server().trim_end_matches('/')),
            user_id: "u1".into(),
            user_name: "mahi".into(),
            token: "tok".into(),
        };
        let http = Arc::new(reqwest_client::ReqwestClient::new());
        let api = Api::new(http, session, "dev".into(), tx);
        let get = |path| futures::executor::block_on(api.get::<serde_json::Value>(path));

        assert!(get("/Expired").unwrap_err().is::<SessionExpired>());
        assert!(matches!(rx.try_next(), Ok(Some(()))), "401 signals expiry");

        assert!(!get("/Missing").unwrap_err().is::<SessionExpired>());
        assert!(rx.try_next().is_err(), "other failures don't signal");

        let segments = futures::executor::block_on(api.segments("ep1")).unwrap();
        assert_eq!(
            segments,
            [
                Segment {
                    kind: SegmentKind::Intro,
                    start: 30.,
                    end: 90.
                },
                Segment {
                    kind: SegmentKind::Outro,
                    start: 1200.,
                    end: 1300.
                },
            ],
            "Intro/Outro only, sorted, in seconds"
        );
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

    #[test]
    fn normalizes_server_input() {
        assert_eq!(
            normalize_server("192.168.1.5:8096/"),
            "http://192.168.1.5:8096"
        );
        assert_eq!(
            normalize_server("  https://jf.example.com//  "),
            "https://jf.example.com"
        );
        assert_eq!(normalize_server("HTTP://host"), "HTTP://host");
        assert_eq!(normalize_server("host/jellyfin/"), "http://host/jellyfin");
    }
}
