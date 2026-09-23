use std::sync::Arc;

use anyhow::Result;
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
    use super::{login, normalize_server};
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::sync::Arc;

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
