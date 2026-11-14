use std::sync::Arc;

use anyhow::Result;
use gpui_kit::http_client::{AsyncBody, HttpClient, Method, Request, Url};
use serde::Deserialize;

use super::types::{PublicUser, Session};
use super::{auth_header, normalize_server, read_json, send};

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

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct QuickConnectResult {
    authenticated: bool,
    secret: String,
    code: String,
}

/// Requests a fresh Quick Connect code; `(server, secret, code)`, `server` normalized
/// so callers don't redo it for the poll/login calls that follow.
pub async fn quick_connect_initiate(
    http: Arc<dyn HttpClient>,
    server_input: &str,
    device_id: &str,
) -> Result<(String, String, String), String> {
    let server = normalize_server(server_input);
    if Url::parse(&server).is_err() {
        return Err(format!("Invalid server URL: {server}"));
    }
    let unreachable = |_| format!("Cannot reach server at {server}.");
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("{server}/QuickConnect/Initiate"))
        .header("Authorization", auth_header(device_id, None))
        .body(AsyncBody::empty())
        .map_err(|_| "Quick Connect request failed.".to_string())?;
    let response = http.send(request).await.map_err(unreachable)?;
    let status = response.status();
    if status.as_u16() == 401 {
        return Err("Quick Connect is turned off on this server.".into());
    }
    if !status.is_success() {
        return Err(format!("Quick Connect failed (HTTP {}).", status.as_u16()));
    }
    let result: QuickConnectResult = read_json(response)
        .await
        .map_err(|_| "Quick Connect failed: unexpected response from server.".to_string())?;
    Ok((server, result.secret, result.code))
}

/// Polls once; `Ok(true)` once the user approved it on another device, `Err` once the
/// code expires or the server otherwise rejects it.
pub async fn quick_connect_poll(
    http: Arc<dyn HttpClient>,
    server: &str,
    secret: &str,
) -> Result<bool, String> {
    let request = Request::builder()
        .uri(format!("{server}/QuickConnect/Connect?secret={secret}"))
        .body(AsyncBody::empty())
        .map_err(|_| "Quick Connect request failed.".to_string())?;
    let response = http
        .send(request)
        .await
        .map_err(|_| "Cannot reach server.".to_string())?;
    let status = response.status();
    if status.as_u16() == 404 {
        return Err("Quick Connect code expired.".into());
    }
    if !status.is_success() {
        return Err(format!("Quick Connect failed (HTTP {}).", status.as_u16()));
    }
    let result: QuickConnectResult = read_json(response)
        .await
        .map_err(|_| "Quick Connect failed: unexpected response from server.".to_string())?;
    Ok(result.authenticated)
}

/// Completes sign-in once `quick_connect_poll` reports approval.
pub async fn quick_connect_login(
    http: Arc<dyn HttpClient>,
    server: &str,
    secret: &str,
    device_id: &str,
) -> Result<Session, String> {
    let body = serde_json::json!({ "Secret": secret }).to_string();
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("{server}/Users/AuthenticateWithQuickConnect"))
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header(device_id, None))
        .body(AsyncBody::from(body))
        .map_err(|_| "Quick Connect failed.".to_string())?;
    let response = http
        .send(request)
        .await
        .map_err(|_| "Cannot reach server.".to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Quick Connect sign-in failed (HTTP {}).",
            status.as_u16()
        ));
    }
    let auth: AuthResult = read_json(response)
        .await
        .map_err(|_| "Quick Connect failed: unexpected response from server.".to_string())?;
    Ok(Session {
        server: server.to_string(),
        user_id: auth.user.id,
        user_name: auth.user.name,
        token: auth.access_token,
    })
}

#[cfg(test)]
mod tests {
    use super::login;
    use crate::jellyfin::tests::fake_server;
    use std::sync::Arc;

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
}
