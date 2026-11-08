//! Tailscale connection screen and locally generated authorization QR.

use super::button::{VegaSize as _, outline_button, primary_button};
use super::layout::centered_scroll;
use super::panel::panel;
use super::typography::{caption, error_text, title};
use crate::design::{gap, overlay, size, text};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{Disableable as _, h_flex, v_flex};
use gpui_kit::*;
use qrcode::{Color, QrCode};
use std::sync::Arc;

pub fn use_tailscale_toggle(
    enabled: bool,
    disabled: bool,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Div {
    h_flex()
        .justify_between()
        .items_center()
        .child(
            div()
                .text_size(text::body())
                .font_weight(FontWeight::MEDIUM)
                .child("Use Tailscale"),
        )
        .child(
            Switch::new("use-tailscale")
                .accessibility_label("Use Tailscale")
                .checked(enabled)
                .disabled(disabled)
                .on_click(move |on, window, cx| on_change(*on, window, cx)),
        )
}

pub fn server_connection_pending(
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Div {
    v_flex()
        .gap(gap::item())
        .child(caption("Connecting to your server…", cx))
        .child(outline_button("cancel-connection", "Cancel connection", cx).on_click(on_cancel))
}

pub fn connect_screen(
    authorization: Option<&Authorization>,
    error: Option<SharedString>,
    on_retry: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Stateful<Div> {
    let mut content = panel(cx)
        .items_center()
        .gap(gap::item())
        .child(title("Connect with Tailscale"));
    if let Some(error) = error {
        content = content.child(error_text(error, cx)).child(
            primary_button("tailscale-retry", "Retry")
                .lg()
                .on_click(on_retry),
        );
    } else if let Some(authorization) = authorization {
        content = content.child(authorization.render(cx));
    } else {
        content = content.child(caption("Connecting to Tailscale…", cx));
    }
    centered_scroll(
        "tailscale-connect",
        v_flex().w(size::form_w()).gap(gap::group()).child(
            content.child(outline_button("tailscale-cancel", "Cancel", cx).on_click(on_cancel)),
        ),
    )
}

pub struct Authorization {
    url: String,
    code: Option<Arc<QrCode>>,
}

impl Authorization {
    pub fn new(url: String) -> Self {
        let code = QrCode::new(url.as_bytes()).ok().map(Arc::new);
        Self { url, code }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn render(&self, cx: &App) -> Div {
        let url = self.url.clone();
        let mut view = v_flex().gap(gap::item()).items_center().child(caption(
            "Authorize this device with Tailscale, then return here.",
            cx,
        ));
        if let Some(code) = self.code.clone() {
            view = view.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, _| {
                        window.paint_quad(fill(bounds, overlay::white(1.)));
                        let module =
                            px((f32::from(bounds.size.width) / (code.width() + 8) as f32).floor());
                        let inset = (bounds.size.width - module * code.width() as f32) / 2.;
                        for y in 0..code.width() {
                            for x in 0..code.width() {
                                if code[(x, y)] == Color::Dark {
                                    let origin = bounds.origin
                                        + point(
                                            inset + module * x as f32,
                                            inset + module * y as f32,
                                        );
                                    window.paint_quad(fill(
                                        Bounds::new(origin, gpui_kit::size(module, module)),
                                        overlay::black(1.),
                                    ));
                                }
                            }
                        }
                    },
                )
                .size(size::auth_qr()),
            );
        }
        view.child(
            outline_button("tailscale-authorize", self.url.clone(), cx)
                .w_full()
                .text_size(text::meta())
                .on_click(move |_, _, cx| cx.open_url(&url)),
        )
        .child(caption("Waiting for authorization or device approval…", cx))
    }
}
