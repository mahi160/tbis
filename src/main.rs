mod app;
mod assets;
mod card;
mod config;
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
mod pip;
mod player;
mod search;
mod series;
mod status;
mod support_dir;
mod theme;

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
            fonts::embed(cx).expect("failed to embed Inter");
            theme::init(cx);

            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-f", app::FocusSearch, None),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            player::bind_keys(cx);
            // single-window app: closing window quits
            cx.on_window_closed(|cx, _| cx.quit()).detach();

            cx.spawn(async move |cx| {
                let options = WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(
                        cx.update(|cx| Bounds::centered(None, size(px(1200.), px(800.)), cx)),
                    )),
                    window_min_size: Some(size(px(800.), px(560.))),
                    ..TitleBar::window_options()
                };
                cx.open_window(options, |window, cx| {
                    let view = cx.new(|cx| app::AppView::new(window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("failed to open window");
                cx.update(|cx| cx.activate(true));
            })
            .detach();
        });
}
