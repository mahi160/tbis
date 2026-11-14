use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::channel::mpsc::UnboundedSender;
use gpui_kit::http_client::{AsyncBody, HttpClient, Method, Request, Response};
use serde::{Deserialize, de::DeserializeOwned};

use super::types::{
    ExternalSubtitle, Item, Kind, Library, PlaybackItem, Report, Segment, SegmentKind, Session,
    Sort, Stream, TrickplayInfo, seconds_to_ticks, ticks_to_seconds,
};
use super::{HttpStatus, SessionExpired, auth_header, read_json, send};

/// Tags needed by `Api::wide_image_url`.
const WIDE_IMAGES: &str = "EnableImageTypes=Primary,Thumb,Backdrop&ImageTypeLimit=1";

/// Tags needed by `Api::poster_url`, shared by every plain item listing.
const POSTER_IMAGES: &str = "EnableImageTypes=Primary&ImageTypeLimit=1";

/// Tags needed by `Api::backdrop_url` and `Api::logo_url`, for the Modern view's hero.
const HERO_IMAGES: &str = "EnableImageTypes=Primary,Backdrop,Logo&ImageTypeLimit=1";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PlaybackInfoResult {
    #[serde(default)]
    media_sources: Vec<NegotiatedSource>,
    play_session_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NegotiatedSource {
    id: String,
    #[serde(default)]
    supports_direct_play: bool,
    #[serde(default)]
    supports_direct_stream: bool,
    transcoding_url: Option<String>,
}

/// What mpv can play, for PlaybackInfo negotiation (ported from Photon's profile):
/// anything direct, every HDR range (else the server tonemaps needlessly), subtitles
/// kept as-is, and h264/aac HLS as the transcode target.
fn device_profile(max_bitrate: u64) -> serde_json::Value {
    let ranges = "SDR|HDR10|HDR10Plus|HLG|DOVI|DOVIWithHDR10|DOVIWithHLG|DOVIWithSDR|DOVIWithEL|DOVIWithHDR10Plus|DOVIWithELHDR10Plus";
    let codecs = [
        "h264",
        "hevc",
        "vp8",
        "vp9",
        "av1",
        "mpeg2video",
        "mpeg4",
        "vc1",
    ];
    serde_json::json!({
        "Name": "tbis",
        "MaxStreamingBitrate": max_bitrate,
        "CodecProfiles": codecs.map(|codec| serde_json::json!({
            "Type": "Video",
            "Codec": codec,
            "Conditions": [{
                "Condition": "EqualsAny",
                "Property": "VideoRangeType",
                "Value": ranges,
                "IsRequired": false,
            }],
        })),
        "DirectPlayProfiles": [{ "Type": "Video" }, { "Type": "Audio" }],
        "TranscodingProfiles": [{
            "Container": "mp4",
            "Type": "Video",
            "VideoCodec": "h264",
            "AudioCodec": "aac,mp3",
            "Protocol": "hls",
            "Context": "Streaming",
            "MaxAudioChannels": "8",
            "MinSegments": 2,
            "BreakOnNonKeyFrames": true,
        }],
        "SubtitleProfiles": [
            { "Format": "srt", "Method": "External" },
            { "Format": "pgssub", "Method": "Embed" },
            { "Format": "dvdsub", "Method": "Embed" },
            { "Format": "dvbsub", "Method": "Embed" },
            { "Format": "ass", "Method": "Embed" },
            { "Format": "ssa", "Method": "Embed" },
        ],
    })
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

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemsResult {
    items: Vec<Item>,
}

/// `/UserViews` entry: a Library, Live TV, Collections, a playlist folder, ...
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct UserView {
    id: String,
    name: String,
    collection_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct UserViewsResult {
    items: Vec<UserView>,
}

impl UserViewsResult {
    /// Movie and Series Libraries only, in server order.
    #[allow(dead_code)] // used by the Modern view (roadmap 31)
    fn libraries(self) -> Vec<Library> {
        self.items
            .into_iter()
            .filter_map(|view| {
                let kind = match view.collection_type.as_deref()? {
                    "movies" => Kind::Movie,
                    "tvshows" => Kind::Series,
                    _ => return None,
                };
                Some(Library {
                    id: view.id,
                    name: view.name,
                    kind,
                })
            })
            .collect()
    }
}

/// Signed-in client; every request carries the session token.
#[derive(Clone)]
pub struct Api {
    http: Arc<dyn HttpClient>,
    pub(super) session: Session,
    /// Server in use: base of every request and URL; never written to config.
    pub(super) server: String,
    device_id: String,
    /// Fired on every 401 so the app can drop the session and return to sign-in.
    expired: UnboundedSender<()>,
}

impl Api {
    pub fn new(
        http: Arc<dyn HttpClient>,
        session: Session,
        server: String,
        device_id: String,
        expired: UnboundedSender<()>,
    ) -> Self {
        Self {
            http,
            session,
            server,
            device_id,
            expired,
        }
    }

    pub(super) async fn send(&self, request: Request<AsyncBody>) -> Result<Response<AsyncBody>> {
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
            .uri(format!("{}{path_and_query}", self.server))
            .header("Authorization", self.auth_header())
            .body(AsyncBody::empty())?;
        read_json(self.send(request).await?).await
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> Result<()> {
        self.request(Method::POST, path, body).await
    }

    async fn request(&self, method: Method, path: &str, body: serde_json::Value) -> Result<()> {
        self.send(self.json_request(method, path, body)?).await?;
        Ok(())
    }

    fn json_request(
        &self,
        method: Method,
        path: &str,
        body: serde_json::Value,
    ) -> Result<Request<AsyncBody>> {
        Ok(Request::builder()
            .method(method)
            .uri(format!("{}{path}", self.server))
            .header("Content-Type", "application/json")
            .header("Authorization", self.auth_header())
            .body(AsyncBody::from(body.to_string()))?)
    }

    /// `Items?...&{extra}`, with the `Fields`/image params every item listing shares.
    async fn items(&self, kind: Kind, extra: &str) -> Result<Vec<Item>> {
        self.items_with(kind, POSTER_IMAGES, extra).await
    }

    /// `items` asking for the image tags `images` names instead of the poster's.
    async fn items_with(&self, kind: Kind, images: &str, extra: &str) -> Result<Vec<Item>> {
        let query = format!(
            "/Items?userId={}&IncludeItemTypes={}&Recursive=true&Fields=ProductionYear&{images}&{extra}",
            self.session.user_id,
            kind.as_str(),
        );
        Ok(self.get::<ItemsResult>(&query).await?.items)
    }

    pub fn auth_header(&self) -> String {
        auth_header(&self.device_id, Some(&self.session.token))
    }

    /// Play details plus how to stream them: direct play unless `max_mbps` caps the
    /// bitrate, in which case the server decides (and may hand back a transcode).
    pub async fn playback_item(
        &self,
        item_id: &str,
        max_mbps: Option<u32>,
    ) -> Result<PlaybackItem> {
        let mut item: PlaybackItem = self
            .get(&format!(
                "/Items/{item_id}?userId={}&Fields=Chapters,Trickplay",
                self.session.user_id
            ))
            .await?;
        item.stream = match max_mbps {
            None => self.direct_stream(&item),
            Some(mbps) => self.negotiate_stream(&item, mbps).await?,
        };
        Ok(item)
    }

    fn direct_stream(&self, item: &PlaybackItem) -> Stream {
        Stream {
            url: format!(
                "{}/Videos/{}/stream?static=true&mediaSourceId={}",
                self.server,
                item.id,
                item.media_source_id()
            ),
            transcode: None,
        }
    }

    /// Asks the server (PlaybackInfo) how to play `item` under `mbps`: direct when
    /// the source fits, else its HLS transcode.
    async fn negotiate_stream(&self, item: &PlaybackItem, mbps: u32) -> Result<Stream> {
        let bitrate = u64::from(mbps) * 1_000_000;
        let body = serde_json::json!({
            "UserId": self.session.user_id,
            "MaxStreamingBitrate": bitrate,
            "DeviceProfile": device_profile(bitrate),
            // 0, not the resume point: keeps the transcode on the file's own timeline,
            // so mpv's resume seek, progress reports, and sidecar subtitles line up
            "StartTimeTicks": 0,
            "IsPlayback": true,
            "AutoOpenLiveStream": false,
            "MediaSourceId": item.media_source_id(),
        });
        let request = self.json_request(
            Method::POST,
            &format!("/Items/{}/PlaybackInfo", item.id),
            body,
        )?;
        let info: PlaybackInfoResult = read_json(self.send(request).await?).await?;
        let source = info
            .media_sources
            .iter()
            .find(|s| s.id == item.media_source_id())
            .or(info.media_sources.first())
            .ok_or_else(|| anyhow!("Server offered no way to play this item"))?;
        if source.supports_direct_play || source.supports_direct_stream {
            return Ok(self.direct_stream(item));
        }
        let url = source
            .transcoding_url
            .as_ref()
            .ok_or_else(|| anyhow!("Server can't stream this item under the bitrate cap"))?;
        Ok(Stream {
            url: format!("{}{url}", self.server),
            transcode: Some(info.play_session_id.unwrap_or_default()),
        })
    }

    /// Ends the server's ffmpeg job for a transcode; Stopped alone may leave it running.
    async fn stop_transcode(&self, play_session_id: &str) -> Result<()> {
        let path = format!(
            "/Videos/ActiveEncodings?deviceId={}&playSessionId={play_session_id}",
            self.device_id
        );
        self.request(Method::DELETE, &path, serde_json::Value::Null)
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
            .map(|s| ExternalSubtitle {
                // Not the item's `DeliveryUrl` (often absent outright on this generic
                // detail endpoint, unlike PlaybackInfo): it also bakes in the resume
                // position as the subtitle's own zero point, so on a resumed item every
                // cue would land before mpv's real (absolute-timeline) seek target.
                url: format!(
                    "{}/Videos/{}/{}/Subtitles/{}/Stream.srt",
                    self.server,
                    item.id,
                    item.media_source_id(),
                    s.index
                ),
                title: s.display_title.clone().unwrap_or_default(),
                lang: s.language.clone().unwrap_or_default(),
                is_default: s.is_default,
            })
            .collect()
    }

    /// Widest scrub-preview sprite sheet the server generated for this item, if any.
    pub fn trickplay(&self, item: &PlaybackItem) -> Option<TrickplayInfo> {
        item.trickplay()
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
        // a transcode's reports must carry the server's own session to reach its job
        let session = item.stream.transcode.as_deref().unwrap_or(play_session_id);
        let body = serde_json::json!({
            "ItemId": item.id,
            "MediaSourceId": item.media_source_id(),
            "PlaySessionId": session,
            "PositionTicks": seconds_to_ticks(seconds),
            "IsPaused": paused,
            "PlayMethod": if item.stream.transcode.is_some() { "Transcode" } else { "DirectPlay" },
            "CanSeek": true,
        });
        self.post(path, body).await?;
        if let (Report::Stopped, Some(session)) = (report, &item.stream.transcode) {
            self.stop_transcode(session).await?;
        }
        Ok(())
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

    /// Random items of `kind` (Movies and Series when `None`) that have a backdrop, for
    /// page backdrops.
    pub async fn random_backdrops(&self, kind: Option<Kind>, limit: usize) -> Result<Vec<Item>> {
        let path = format!(
            "/Items?userId={}&IncludeItemTypes={}&Recursive=true&SortBy=Random&ImageTypes=Backdrop&EnableImageTypes=Backdrop&ImageTypeLimit=1&EnableUserData=false&Limit={limit}",
            self.session.user_id,
            kind.map_or("Movie,Series", Kind::as_str),
        );
        Ok(self.get::<ItemsResult>(&path).await?.items)
    }

    /// Every played Movie and Episode (no images), for Watch stats.
    pub async fn played_items(&self) -> Result<Vec<Item>> {
        let path = format!(
            "/Items?userId={}&Recursive=true&IncludeItemTypes=Movie,Episode&Filters=IsPlayed&EnableImages=false",
            self.session.user_id
        );
        Ok(self.get::<ItemsResult>(&path).await?.items)
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
            "/Shows/{series_id}/Episodes?userId={}&seasonId={season_id}&Fields=Overview&EnableImageTypes=Primary&ImageTypeLimit=1",
            self.session.user_id
        );
        Ok(self.get::<ItemsResult>(&path).await?.items)
    }

    /// Episodes before and after `episode_id` in the server's order, crossing seasons.
    pub async fn adjacent_episodes(
        &self,
        series_id: &str,
        episode_id: &str,
    ) -> Result<(Option<Item>, Option<Item>)> {
        let path = format!(
            "/Shows/{series_id}/Episodes?userId={}&{WIDE_IMAGES}",
            self.session.user_id
        );
        let mut episodes = self.get::<ItemsResult>(&path).await?.items;
        let Some(at) = episodes.iter().position(|e| e.id == episode_id) else {
            return Ok((None, None));
        };
        let next = (at + 1 < episodes.len()).then(|| episodes.remove(at + 1));
        let previous = at.checked_sub(1).map(|ix| episodes.remove(ix));
        Ok((previous, next))
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

    /// Movie and Series Libraries, in server order.
    #[allow(dead_code)] // used by the Modern view (roadmap 31)
    pub async fn libraries(&self) -> Result<Vec<Library>> {
        let path = format!("/UserViews?userId={}", self.session.user_id);
        Ok(self.get::<UserViewsResult>(&path).await?.libraries())
    }

    /// Newest items of one Library, with the tags the hero's backdrop and logo need.
    #[allow(dead_code)] // used by the Modern view (roadmap 31)
    pub async fn latest_in(&self, library: &Library, limit: usize) -> Result<Vec<Item>> {
        self.items_with(
            library.kind,
            HERO_IMAGES,
            &format!(
                "ParentId={}&Limit={limit}&{}",
                library.id,
                Sort::DateAdded.query(library.kind)
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
}

#[cfg(test)]
mod tests {
    use super::{Api, UserViewsResult};
    use crate::jellyfin::tests::fake_server;
    use crate::jellyfin::{
        Item, Kind, Library, Person, PlaybackItem, Segment, SegmentKind, Session, SessionExpired,
        TrickplayInfo,
    };
    use gpui_kit::http_client::Method;
    use std::sync::Arc;

    #[test]
    fn libraries_keep_movies_and_series_in_server_order() {
        // trimmed from a real /UserViews response
        let views: UserViewsResult = serde_json::from_value(serde_json::json!({"Items": [
            {"Id": "c", "Name": "Collections", "CollectionType": "boxsets"},
            {"Id": "n", "Name": "New", "CollectionType": null},
            {"Id": "tv", "Name": "TV Shows", "CollectionType": "tvshows"},
            {"Id": "l", "Name": "Live TV", "CollectionType": "livetv"},
            {"Id": "m", "Name": "Movies", "CollectionType": "movies"},
        ]}))
        .unwrap();
        let library = |id: &str, name: &str, kind| Library {
            id: id.into(),
            name: name.into(),
            kind,
        };
        assert_eq!(
            views.libraries(),
            [
                library("tv", "TV Shows", Kind::Series),
                library("m", "Movies", Kind::Movie),
            ]
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
        let server = session.server.clone();
        let api = Api::new(http, session, server, "dev".into(), tx);
        let get = |path| futures::executor::block_on(api.get::<serde_json::Value>(path));

        assert!(get("/Expired").unwrap_err().is::<SessionExpired>());
        assert!(matches!(rx.try_recv(), Ok(())), "401 signals expiry");

        assert!(!get("/Missing").unwrap_err().is::<SessionExpired>());
        assert!(rx.try_recv().is_err(), "other failures don't signal");

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
    fn urls_follow_server_in_use() {
        let (tx, _rx) = futures::channel::mpsc::unbounded();
        let session = Session {
            // nothing listens here: a request built from the saved address fails
            server: "http://127.0.0.1:1".into(),
            user_id: "u1".into(),
            user_name: "mahi".into(),
            token: "tok".into(),
        };
        let in_use = format!("http://{}", fake_server().trim_end_matches('/'));
        let http = Arc::new(reqwest_client::ReqwestClient::new());
        let api = Api::new(http, session, in_use.clone(), "dev".into(), tx);

        let segments = futures::executor::block_on(api.segments("ep1"));
        assert_eq!(
            segments.unwrap().len(),
            2,
            "requests go to the server in use"
        );

        let item: Item = serde_json::from_value(serde_json::json!({
            "Id": "i", "Name": "Movie",
            "ImageTags": {"Primary": "p"}, "BackdropImageTags": ["b"],
        }))
        .unwrap();
        let person: Person = serde_json::from_value(
            serde_json::json!({"Id": "p1", "Name": "A", "PrimaryImageTag": "t"}),
        )
        .unwrap();
        let playback: PlaybackItem = serde_json::from_value(serde_json::json!({
            "Id": "i",
            "MediaSources": [{"Id": "m", "MediaStreams": [{"Type": "Subtitle", "Index": 2, "IsExternal": true}]}],
        }))
        .unwrap();
        let trickplay: TrickplayInfo = serde_json::from_value(serde_json::json!({
            "Width": 320, "Height": 180, "TileWidth": 10, "TileHeight": 10,
            "ThumbnailCount": 100, "Interval": 10000,
        }))
        .unwrap();

        let urls = [
            api.poster_url(&item).unwrap(),
            api.backdrop_url(&item).unwrap(),
            api.wide_image_url(&item).unwrap(),
            api.person_image_url(&person).unwrap(),
            api.direct_stream(&playback).url,
            api.trickplay_tile_url(&playback, &trickplay, 0),
            api.external_subtitles(&playback)[0].url.clone(),
            api.json_request(Method::POST, "/x", serde_json::Value::Null)
                .unwrap()
                .uri()
                .to_string(),
        ];
        for url in urls {
            assert!(url.starts_with(&format!("{in_use}/")), "{url}");
        }
    }
}
