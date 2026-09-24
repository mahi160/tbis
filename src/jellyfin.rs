use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, ensure};
use futures::AsyncReadExt as _;
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Item {
    pub id: String,
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
}

impl Item {
    /// `S2E3 · Name`, or just the name when numbers are missing.
    pub fn episode_label(&self) -> String {
        match (self.parent_index_number, self.index_number) {
            (Some(season), Some(episode)) => {
                format!("S{season}E{episode} · {}", self.display_name())
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

/// Item type searched on its own, so one type can't crowd out the others.
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Movie,
    Series,
    Episode,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Movie => "Movie",
            Kind::Series => "Series",
            Kind::Episode => "Episode",
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
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MediaSource {
    id: String,
}

impl PlaybackItem {
    pub fn media_source_id(&self) -> &str {
        self.media_sources.first().map_or(&self.id, |m| &m.id)
    }

    pub fn resume_seconds(&self) -> f64 {
        ticks_to_seconds(self.user_data.playback_position_ticks)
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
}

impl Api {
    pub fn new(http: Arc<dyn HttpClient>, session: Session, device_id: String) -> Self {
        Self {
            http,
            session,
            device_id,
        }
    }

    async fn get<T: DeserializeOwned>(&self, path_and_query: &str) -> Result<T> {
        let request = Request::builder()
            .uri(format!("{}{path_and_query}", self.session.server))
            .header("Authorization", self.auth_header())
            .body(AsyncBody::empty())?;
        let response = self.http.send(request).await?;
        ensure!(
            response.status().is_success(),
            "HTTP {}",
            response.status().as_u16()
        );
        read_json(response).await
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> Result<()> {
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("{}{path}", self.session.server))
            .header("Content-Type", "application/json")
            .header("Authorization", self.auth_header())
            .body(AsyncBody::from(body.to_string()))?;
        let response = self.http.send(request).await?;
        ensure!(
            response.status().is_success(),
            "HTTP {}",
            response.status().as_u16()
        );
        Ok(())
    }

    pub fn auth_header(&self) -> String {
        auth_header(&self.device_id, Some(&self.session.token))
    }

    pub async fn playback_item(&self, item_id: &str) -> Result<PlaybackItem> {
        self.get(&format!("/Items/{item_id}?userId={}", self.session.user_id))
            .await
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

    pub async fn mark_played(&self, item_id: &str) -> Result<()> {
        let path = format!("/Users/{}/PlayedItems/{item_id}", self.session.user_id);
        self.post(&path, serde_json::Value::Null).await
    }

    /// Every Movie from all Libraries.
    pub async fn library(&self, kind: Kind, sort: Sort) -> Result<Vec<Item>> {
        let query = format!(
            "/Items?userId={}&IncludeItemTypes={}&Recursive=true&{}&Fields=ProductionYear&EnableImageTypes=Primary&ImageTypeLimit=1",
            self.session.user_id,
            kind.as_str(),
            sort.query(kind)
        );
        Ok(self.get::<ItemsResult>(&query).await?.items)
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

    /// Next unwatched Episode of one Series, if any.
    /// Episode after `episode_id` in the server's order, crossing seasons.
    pub async fn next_episode(&self, series_id: &str, episode_id: &str) -> Result<Option<Item>> {
        let path = format!(
            "/Shows/{series_id}/Episodes?userId={}&startItemId={episode_id}&Limit=2",
            self.session.user_id
        );
        let episodes = self.get::<ItemsResult>(&path).await?.items;
        Ok(episodes.into_iter().find(|e| e.id != episode_id))
    }

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

    /// Server-side search across all Libraries, one type at a time.
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
        let query = format!(
            "/Items?userId={}&IncludeItemTypes={}&Recursive=true&isPlayed=false&Limit={limit}&{}&Fields=ProductionYear&EnableImageTypes=Primary&ImageTypeLimit=1",
            self.session.user_id,
            kind.as_str(),
            Sort::DateAdded.query(kind)
        );
        Ok(self.get::<ItemsResult>(&query).await?.items)
    }

    pub async fn search(&self, term: &str, kind: Kind, limit: usize) -> Result<Vec<Item>> {
        let term: String = url::form_urlencoded::byte_serialize(term.as_bytes()).collect();
        let query = format!(
            "/Items?userId={}&searchTerm={term}&IncludeItemTypes={}&Recursive=true&Limit={limit}&Fields=ProductionYear&EnableImageTypes=Primary&ImageTypeLimit=1",
            self.session.user_id,
            kind.as_str()
        );
        Ok(self.get::<ItemsResult>(&query).await?.items)
    }

    /// 16:9 art. Episode: still, Series thumb, Series backdrop. Movie: thumb, backdrop, poster.
    pub fn wide_image_url(&self, item: &Item) -> Option<String> {
        let own = |kind: &'static str, tag: Option<&String>| {
            tag.map(|t| (item.id.clone(), kind, t.clone()))
        };
        let parent = |kind: &'static str, id: &Option<String>, tag: Option<&String>| {
            id.clone().zip(tag.cloned()).map(|(id, t)| (id, kind, t))
        };
        let pick = if item.series_name.is_some() {
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
    let response = http.send(request).await?;
    anyhow::ensure!(response.status().is_success(), "HTTP {}", response.status());
    read_json(response).await
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
    use super::{Item, login, normalize_server};
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
            "S4E15 · Night Out"
        );
        assert_eq!(
            label("The Office (US) - S04E18-19 - Goodbye, Toby", 4, 18),
            "S4E18 · Goodbye, Toby"
        );
        assert_eq!(
            label("A - B - s4e15 - Title - Part 2", 4, 15),
            "S4E15 · Title - Part 2"
        );
        // code for another episode, or no code: untouched
        assert_eq!(
            label("Show - S04E16 - Night Out", 4, 15),
            "S4E15 · Show - S04E16 - Night Out"
        );
        assert_eq!(
            label("Before - After - End", 1, 1),
            "S1E1 · Before - After - End"
        );
        assert_eq!(label("Show - S04E15 - ", 4, 15), "S4E15 · Show - S04E15 - ");
        assert_eq!(label("Pilot", 1, 1), "S1E1 · Pilot");
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
