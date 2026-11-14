//! Jellyfin server client: `types` holds the data, `api` the signed-in client,
//! `images` its image URLs, and `auth` sign-in. This file holds the shared request helpers.

mod api;
mod auth;
mod images;
mod types;

pub use api::Api;
pub use auth::{
    login, public_users, quick_connect_initiate, quick_connect_login, quick_connect_poll,
};
pub use images::blurred;
pub use types::*;

use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::AsyncReadExt as _;
use gpui_kit::http_client::{AsyncBody, HttpClient, Request, Response, Url};
use serde::de::DeserializeOwned;

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

/// True when `server` (already normalized) is plain HTTP to a host outside private
/// networks: sign-in details would cross the internet unencrypted.
pub fn is_unencrypted_remote(server: &str) -> bool {
    let Ok(url) = Url::parse(server) else {
        return false;
    };
    if url.scheme() != "http" {
        return false;
    }
    match url.host() {
        Some(url::Host::Ipv4(ip)) => {
            // 100.64.0.0/10 is carrier-grade NAT space, used by Tailscale
            let cgnat = ip.octets()[0] == 100 && (64..128).contains(&ip.octets()[1]);
            !(ip.is_private() || ip.is_loopback() || ip.is_link_local() || cgnat)
        }
        Some(url::Host::Ipv6(ip)) => !(ip.is_loopback() || ip.is_unique_local()),
        Some(url::Host::Domain(name)) => {
            let name = name.to_ascii_lowercase();
            // a bare name (no dot) or .local/.lan/.home is a machine on the local network
            name.contains('.')
                && name != "localhost"
                && ![".local", ".lan", ".home", ".internal", ".ts.net"]
                    .iter()
                    .any(|suffix| name.ends_with(suffix))
        }
        None => false,
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

#[cfg(test)]
mod tests {
    use super::{is_unencrypted_remote, normalize_server};
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    /// Minimal fake Jellyfin: accepts password "right", or empty for passwordless "guest".
    pub(super) fn fake_server() -> String {
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

    #[test]
    fn flags_only_unencrypted_public_servers() {
        for (server, expected) in [
            ("http://192.168.1.5:8096", false),
            ("http://100.100.1.2:8096", false),
            ("http://nas:8096", false),
            ("http://jellyfin.local", false),
            ("https://example.com", false),
            ("http://example.com:8096", true),
            ("http://8.8.8.8", true),
        ] {
            assert_eq!(is_unencrypted_remote(server), expected, "{server}");
        }
    }
}
