mod app;
mod config;
mod jellyfin;
mod login;
mod movies;

use std::sync::Arc;

use gpui_kit::component::{Root, Theme, ThemeMode, TitleBar};
use gpui_kit::*;

actions!(tbis, [Quit]);

fn main() {
    let http =
        reqwest_client::ReqwestClient::user_agent(concat!("tbis/", env!("CARGO_PKG_VERSION")))
            .expect("failed to build HTTP client");

    gpui_kit::application()
        .with_http_client(Arc::new(http))
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);

            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            cx.on_action(|_: &Quit, cx| cx.quit());
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
