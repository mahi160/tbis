//! One route per launch/sign-in; the saved LAN address is never replaced by loopback.

use crate::{config::TailscaleOptions, jellyfin};
use anyhow::{Result, anyhow};
use futures::future::{Either, select};
use gpui_kit::{BackgroundExecutor, http_client::HttpClient};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
pub use crate::tailnet::Service;

#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
pub struct Service;

#[cfg(not(target_os = "macos"))]
impl Service {
    pub fn new() -> Self {
        Self
    }
    pub fn status_label(&self) -> Result<&'static str> {
        Ok("off")
    }
    pub fn log_out(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteKind {
    Lan,
    Tailnet,
}

impl RouteKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lan => "LAN",
            Self::Tailnet => "tailnet",
        }
    }
}

#[derive(Debug)]
pub struct NeedsAuthorization;

impl std::fmt::Display for NeedsAuthorization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Tailscale needs authorization.")
    }
}

impl std::error::Error for NeedsAuthorization {}

#[derive(Clone)]
pub struct Route {
    pub server: String,
    pub kind: RouteKind,
}

async fn choose_route(
    lan: impl Future<Output = Result<String>>,
    enabled: bool,
    tailnet: impl Future<Output = Result<String>>,
) -> Result<Route> {
    match lan.await {
        Ok(server) => Ok(Route {
            server,
            kind: RouteKind::Lan,
        }),
        Err(_) if enabled => Ok(Route {
            server: tailnet.await?,
            kind: RouteKind::Tailnet,
        }),
        Err(err) => Err(err),
    }
}

async fn probe(
    http: Arc<dyn HttpClient>,
    server: &str,
    executor: &BackgroundExecutor,
    timeout: Duration,
) -> Result<String> {
    let request = Box::pin(jellyfin::probe(http, server));
    match select(request, executor.timer(timeout)).await {
        Either::Left((result, _)) => {
            result?;
            Ok(server.to_owned())
        }
        Either::Right(_) => Err(anyhow!("Server connection timed out.")),
    }
}

pub async fn resolve(
    service: &Service,
    http: Arc<dyn HttpClient>,
    executor: &BackgroundExecutor,
    saved_server: &str,
    options: &TailscaleOptions,
) -> Result<Route> {
    let server = jellyfin::normalize_server(saved_server);
    choose_route(
        probe(http.clone(), &server, executor, Duration::from_millis(1500)),
        options.enabled,
        connect_tailnet(service, http, executor, &options.address, &server),
    )
    .await
}

#[cfg(target_os = "macos")]
pub async fn authorize(
    service: &Service,
    executor: &BackgroundExecutor,
    auth_url: impl FnMut(Option<String>),
) -> Result<()> {
    wait_for_node(service, executor, true, auth_url).await
}

#[cfg(target_os = "macos")]
async fn wait_for_node(
    service: &Service,
    executor: &BackgroundExecutor,
    interactive: bool,
    mut auth_url: impl FnMut(Option<String>),
) -> Result<()> {
    use crate::tailscale::BackendState;
    let deadline = Instant::now() + Duration::from_secs(if interactive { 300 } else { 15 });
    loop {
        let service = service.clone();
        let status = executor
            .spawn(async move { service.start_status() })
            .await?;
        if status.backend_state == BackendState::Running {
            break;
        }
        if !interactive
            && (status.backend_state == BackendState::NeedsMachineAuth
                || (status.backend_state == BackendState::NeedsLogin && status.auth_url.is_some()))
        {
            return Err(NeedsAuthorization.into());
        }
        auth_url(status.auth_url);
        if Instant::now() >= deadline {
            return Err(anyhow!(
                "Tailscale authorization timed out. Try connecting again."
            ));
        }
        executor.timer(Duration::from_millis(500)).await;
    }
    auth_url(None);
    Ok(())
}

/// Forwarder target from the tailnet address; the saved server only contributes its path prefix.
#[cfg(target_os = "macos")]
fn tailnet_destination(address: &str, server: &str) -> Result<(String, String)> {
    let address = address.trim();
    if address.is_empty() {
        return Err(anyhow!("Enter a tailnet address."));
    }
    let address = crate::tailscale::parse_address(address)?;
    let path = url::Url::parse(server)?
        .path()
        .trim_end_matches('/')
        .to_owned();
    Ok((address, path))
}

#[cfg(target_os = "macos")]
async fn connect_tailnet(
    service: &Service,
    http: Arc<dyn HttpClient>,
    executor: &BackgroundExecutor,
    address: &str,
    server: &str,
) -> Result<String> {
    let (address, path) = tailnet_destination(address, server)?;
    wait_for_node(service, executor, false, |_| {}).await?;
    let service = service.clone();
    let server = executor
        .spawn(async move { service.forward(&address) })
        .await?;
    probe(
        http,
        &format!("{server}{path}"),
        executor,
        Duration::from_secs(15),
    )
    .await
}

#[cfg(not(target_os = "macos"))]
async fn connect_tailnet(
    _: &Service,
    _: Arc<dyn HttpClient>,
    _: &BackgroundExecutor,
    _: &str,
    _: &str,
) -> Result<String> {
    Err(anyhow!("Embedded Tailscale is supported only on macOS."))
}

#[cfg(not(target_os = "macos"))]
pub async fn authorize(
    _: &Service,
    _: &BackgroundExecutor,
    _: impl FnMut(Option<String>),
) -> Result<()> {
    Err(anyhow!("Embedded Tailscale is supported only on macOS."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use std::cell::Cell;

    #[cfg(target_os = "macos")]
    #[test]
    fn tailnet_address_with_saved_server_path_is_the_destination() {
        assert_eq!(
            tailnet_destination("jellyfin:8096", "http://192.168.1.5/jellyfin/").unwrap(),
            ("jellyfin:8096".into(), "/jellyfin".into())
        );
        assert_eq!(
            tailnet_destination("100.64.0.1", "http://192.168.1.5:8096").unwrap(),
            ("100.64.0.1:8096".into(), String::new())
        );
        assert!(tailnet_destination("", "http://192.168.1.5").is_err());
        assert!(tailnet_destination("192.168.1.5", "http://192.168.1.5").is_err());
    }

    #[test]
    fn lan_success_never_starts_tailnet() {
        let started = Cell::new(false);
        let route = block_on(choose_route(
            async { Ok("http://lan".into()) },
            true,
            async {
                started.set(true);
                Ok("http://127.0.0.1:1234".into())
            },
        ))
        .unwrap();
        assert_eq!(route.kind, RouteKind::Lan);
        assert_eq!(route.server, "http://lan");
        assert!(!started.get());
    }

    #[test]
    fn lan_timeout_uses_tailnet_only_when_enabled() {
        for enabled in [true, false] {
            let started = Cell::new(false);
            let result = block_on(choose_route(
                async { Err(anyhow!("LAN timed out")) },
                enabled,
                async {
                    started.set(true);
                    Ok("http://127.0.0.1:1234".into())
                },
            ));
            assert_eq!(started.get(), enabled);
            if enabled {
                let route = result.unwrap();
                assert_eq!(route.kind, RouteKind::Tailnet);
                assert_eq!(route.server, "http://127.0.0.1:1234");
            } else {
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn both_routes_fail_without_returning_a_dead_route() {
        assert!(
            block_on(choose_route(
                async { Err(anyhow!("LAN timed out")) },
                true,
                async { Err(anyhow!("tailnet unavailable")) }
            ))
            .is_err()
        );
    }
}
