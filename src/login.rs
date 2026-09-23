use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, v_flex};
use gpui_kit::*;

use crate::jellyfin::{self, Session};

pub struct LoggedIn(pub Session);

pub struct LoginView {
    server: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    device_id: String,
    error: Option<SharedString>,
    busy: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<LoggedIn> for LoginView {}

impl LoginView {
    pub fn new(device_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let server = cx.new(|cx| InputState::new(window, cx).placeholder("192.168.1.5:8096"));
        let username = cx.new(|cx| InputState::new(window, cx).placeholder("Username"));
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Password (optional)")
                .masked(true)
        });
        let _subscriptions = [&server, &username, &password]
            .into_iter()
            .map(|input| {
                cx.subscribe_in(input, window, |this, _, event, window, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        this.submit(window, cx);
                    }
                })
            })
            .collect();
        server.update(cx, |input, cx| input.focus(window, cx));

        Self {
            server,
            username,
            password,
            device_id,
            error: None,
            busy: false,
            _subscriptions,
        }
    }

    fn submit(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let server = self.server.read(cx).value().to_string();
        let username = self.username.read(cx).value().to_string();
        let password = self.password.read(cx).value().to_string();
        let device_id = self.device_id.clone();
        let http = cx.http_client();

        self.busy = true;
        self.error = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = jellyfin::login(http, &server, &username, &password, &device_id).await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(session) => cx.emit(LoggedIn(session)),
                    Err(message) => this.error = Some(message.into()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Render for LoginView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let field = |label: &'static str, input: &Entity<InputState>| {
            v_flex()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(label),
                )
                .child(Input::new(input).disabled(self.busy))
        };

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .w(px(360.))
                    .gap_4()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Sign in to Jellyfin"),
                    )
                    .child(field("Server", &self.server))
                    .child(field("Username", &self.username))
                    .child(field("Password", &self.password))
                    .children(
                        self.error.clone().map(|error| {
                            div().text_sm().text_color(cx.theme().danger).child(error)
                        }),
                    )
                    .child(
                        Button::new("sign-in")
                            .primary()
                            .label("Sign in")
                            .loading(self.busy)
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, window, cx| this.submit(window, cx))),
                    ),
            )
    }
}
