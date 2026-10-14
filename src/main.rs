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

actions!(tbis, [Quit]);

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
                eprintln!(
                    "could not raise open file limit: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
    }
}

fn main() {
    #[cfg(unix)]
    raise_open_file_limit();

    let http =
        reqwest_client::ReqwestClient::user_agent(concat!("tbis/", env!("CARGO_PKG_VERSION")))
            .expect("failed to build HTTP client");

    gpui_kit::application()
        .with_http_client(Arc::new(http))
        .with_assets(assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            fonts::embed(cx).expect("failed to embed fonts");
            theme::init(config::load().theme.as_deref(), cx);

            cx.bind_keys([
                // secondary: cmd on macOS, ctrl elsewhere
                KeyBinding::new("secondary-q", Quit, None),
                KeyBinding::new("secondary-f", app::FocusSearch, None),
                KeyBinding::new("secondary-[", app::GoBack, Some("!Input")),
                KeyBinding::new("secondary-1", app::ShowHome, Some("!Input")),
                KeyBinding::new("secondary-2", app::ShowMovies, Some("!Input")),
                KeyBinding::new("secondary-3", app::ShowSeries, Some("!Input")),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            player::bind_keys(cx);
            shortcuts::bind_keys(cx);

            cx.spawn(async move |cx| {
                let min = design::window::MIN;
                let saved = config::load().window;
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
                        traffic_light_position: Some(design::window::TRAFFIC_LIGHTS),
                        ..TitleBar::title_bar_options()
                    }),
                    ..TitleBar::window_options()
                };
                let main = cx
                    .open_window(options, |window, cx| {
                        platform::window::prepare(window);
                        let view = cx.new(|cx| app::AppView::new(window, cx));
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
