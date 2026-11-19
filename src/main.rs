#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod assets;
mod backdrops;
mod components;
mod config;
mod design;
mod detail;
mod embed;
mod fonts;
mod home;
mod jellyfin;
mod library;
mod log;
mod login;
mod movie;
mod mpv;
mod nav;
mod now_playing;
mod platform;
mod player;
mod search;
mod series;
mod settings;
mod shaders;
mod shortcuts;
mod stats;
mod support_dir;
mod theme;
mod update;

use std::sync::Arc;

use gpui_kit::component::{Root, TitleBar};
use gpui_kit::*;

actions!(tbis, [Quit, CloseWindow, Hide]);

/// Poster grids open hundreds of sockets at once; macOS apps default to 256 files.
#[cfg(unix)]
fn raise_open_file_limit() {
    const WANTED: libc::rlim_t = 10240; // macOS OPEN_MAX
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe {
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) == 0 && limit.rlim_cur < WANTED {
            limit.rlim_cur = WANTED.min(limit.rlim_max);
            if libc::setrlimit(libc::RLIMIT_NOFILE, &limit) != 0 {
                crate::log::write(&format!(
                    "could not raise open file limit: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
    }
}

/// Give up on a response after this long without receiving data.
const HTTP_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

fn http_client() -> reqwest_client::ReqwestClient {
    reqwest_client::ReqwestClient::proxy_user_agent_and_read_timeout(
        None,
        concat!("tbis/", env!("CARGO_PKG_VERSION")),
        Some(HTTP_IDLE_TIMEOUT),
    )
    .expect("failed to build HTTP client")
}

fn main() {
    let _platform = platform::init();

    #[cfg(unix)]
    raise_open_file_limit();

    gpui_kit::application()
        .with_http_client(Arc::new(http_client()))
        .with_assets(assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            fonts::embed(cx).expect("failed to embed fonts");
            let config = config::load();
            theme::init(config.theme.as_deref(), cx);

            cx.bind_keys([
                // secondary: cmd on macOS, ctrl elsewhere
                KeyBinding::new("secondary-q", Quit, None),
                KeyBinding::new("secondary-w", CloseWindow, None),
                #[cfg(target_os = "macos")]
                KeyBinding::new("cmd-h", Hide, None),
                KeyBinding::new("secondary-f", app::FocusSearch, None),
                KeyBinding::new("secondary-[", app::GoBack, Some("!Input")),
                KeyBinding::new("secondary-1", app::ShowHome, Some("!Input")),
                KeyBinding::new("secondary-2", app::ShowMovies, Some("!Input")),
                KeyBinding::new("secondary-3", app::ShowSeries, Some("!Input")),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            // closing the main window quits (below); PiP handles CloseWindow itself
            cx.on_action(|_: &CloseWindow, cx| cx.quit());
            cx.on_action(|_: &Hide, cx| cx.hide());
            player::bind_keys(cx);
            shortcuts::bind_keys(cx);

            cx.spawn(async move |cx| {
                let min = design::window::MIN;
                let saved = config.window;
                let window_bounds = cx.update(|cx| {
                    let displays: Vec<_> = cx.displays().iter().map(|d| d.bounds()).collect();
                    saved
                        .and_then(|w| w.restore(&displays, min))
                        .unwrap_or_else(|| {
                            WindowBounds::Windowed(Bounds::centered(
                                None,
                                design::window::DEFAULT,
                                cx,
                            ))
                        })
                });
                let options = WindowOptions {
                    window_bounds: Some(window_bounds),
                    window_min_size: Some(min),
                    // recentre traffic lights in 52px title bar (app.rs)
                    titlebar: Some(TitlebarOptions {
                        traffic_light_position: design::window::TRAFFIC_LIGHTS,
                        ..TitleBar::title_bar_options()
                    }),
                    ..TitleBar::window_options()
                };
                let main = cx
                    .open_window(options, |window, cx| {
                        platform::window::prepare(window);
                        let view = cx.new(|cx| app::AppView::new(config, window, cx));
                        cx.new(|cx| Root::new(view, window, cx))
                    })
                    .expect("failed to open window");
                cx.update(|cx| {
                    // closing the main window quits; closing PiP does not
                    cx.on_window_closed(move |cx, id| {
                        if id == main.window_id() {
                            cx.quit();
                        }
                    })
                    .detach();
                    cx.activate(true);
                });
            })
            .detach();
        });
}

#[cfg(test)]
mod tests {
    use gpui_kit::http_client::HttpClient as _;
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    #[test]
    fn stalled_http_response_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (release, wait) = std::sync::mpsc::channel::<()>();
        let server = std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            let _ = wait.recv_timeout(Duration::from_secs(45));
        });
        let start = Instant::now();
        let result =
            futures::executor::block_on(super::http_client().get(&url, Default::default(), false));
        let elapsed = start.elapsed();
        let _ = release.send(());
        server.join().unwrap();
        assert!(result.is_err(), "a stalled response must time out");
        assert!(elapsed >= super::HTTP_IDLE_TIMEOUT - Duration::from_secs(1));
        assert!(elapsed < Duration::from_secs(45));
    }
}
