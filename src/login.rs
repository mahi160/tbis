use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use std::time::Duration;

use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::jellyfin::{self, PublicUser, Session};

/// Wait after typing in Server before asking it for users.
const USERS_DEBOUNCE: Duration = Duration::from_millis(400);

pub struct LoggedIn(pub Session);

pub struct LoginView {
    server: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    device_id: String,
    error: Option<SharedString>,
    busy: bool,
    users: Vec<PublicUser>,
    _users: Task<()>,
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
        let mut _subscriptions: Vec<Subscription> = [&server, &username, &password]
            .into_iter()
            .map(|input| {
                cx.subscribe_in(input, window, |this, _, event, window, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        this.submit(window, cx);
                    }
                })
            })
            .collect();
        _subscriptions.push(cx.subscribe(&server, |this, _, event, cx| {
            if let InputEvent::Change = event {
                this.load_users(cx);
            }
        }));
        server.update(cx, |input, cx| input.focus(window, cx));

        Self {
            server,
            username,
            password,
            device_id,
            error: None,
            busy: false,
            users: Vec::new(),
            _users: Task::ready(()),
            _subscriptions,
        }
    }

    /// Shown after the server rejected the saved token: server kept, expiry explained.
    pub fn expired(&mut self, server: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.server.update(cx, |input, cx| {
            input.set_value(server.to_string(), window, cx)
        });
        self.error = Some(jellyfin::SessionExpired.to_string().into());
        cx.notify();
    }

    /// Silent: failures just mean no picker; Sign in reports real errors.
    fn load_users(&mut self, cx: &mut Context<Self>) {
        let server = self.server.read(cx).value().trim().to_string();
        self.users.clear();
        cx.notify();
        if server.is_empty() {
            self._users = Task::ready(());
            return;
        }
        let http = cx.http_client();
        // replacing task cancels older lookup
        self._users = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(USERS_DEBOUNCE).await;
            let users = jellyfin::public_users(http, &server)
                .await
                .unwrap_or_default();
            this.update(cx, |this, cx| {
                this.users = users;
                cx.notify();
            })
            .ok();
        });
    }

    fn pick_user(&mut self, user: &PublicUser, window: &mut Window, cx: &mut Context<Self>) {
        self.username.update(cx, |input, cx| {
            input.set_value(user.name.clone(), window, cx)
        });
        self.password
            .update(cx, |input, cx| input.set_value("", window, cx));
        if user.has_password {
            self.password
                .update(cx, |input, cx| input.focus(window, cx));
        } else {
            self.submit(window, cx);
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
                    .when(!self.users.is_empty(), |this| {
                        this.child(h_flex().flex_wrap().gap_2().children(
                            self.users.iter().enumerate().map(|(i, user)| {
                                let picked = user.clone();
                                Button::new(("user", i))
                                    .outline()
                                    .small()
                                    .label(user.name.clone())
                                    .disabled(self.busy)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.pick_user(&picked, window, cx)
                                    }))
                            }),
                        ))
                    })
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
