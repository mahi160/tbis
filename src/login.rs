use gpui_kit::component::input::{InputEvent, InputState};
use std::time::Duration;

use gpui_kit::component::{Disableable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::components::button::{VegaSize as _, outline_button, primary_button};
use crate::components::input::text_field;
use crate::components::layout::centered;
use crate::components::logo::brand_hero;
use crate::components::panel::panel;
use crate::components::typography::{caption, display, error_text};
use crate::design::{size, sp};
use crate::jellyfin::{self, PublicUser, Session};

/// Wait after typing in Server before asking it for users.
const USERS_DEBOUNCE: Duration = Duration::from_millis(400);
/// How often to ask the server whether the Quick Connect code was approved.
const QUICK_CONNECT_POLL_EVERY: Duration = Duration::from_secs(2);

pub struct LoggedIn(pub Session);

/// A Quick Connect code shown to the user while `LoginView::_quick_connect` polls
/// for approval.
struct QuickConnect {
    code: String,
}

pub struct LoginView {
    server: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    device_id: String,
    error: Option<SharedString>,
    busy: bool,
    users: Vec<PublicUser>,
    quick_connect: Option<QuickConnect>,
    _users: Task<()>,
    _quick_connect: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<LoggedIn> for LoginView {}

impl LoginView {
    pub fn new(device_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let server = cx.new(|cx| InputState::new(window, cx).placeholder("192.168.1.5:8096"));
        let username = cx.new(|cx| InputState::new(window, cx));
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Optional")
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
            quick_connect: None,
            _users: Task::ready(()),
            _quick_connect: Task::ready(()),
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

    /// Requests a code, shows it, then polls until approved or the server rejects it.
    fn quick_connect_start(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.quick_connect.is_some() {
            return;
        }
        let server_input = self.server.read(cx).value().to_string();
        let device_id = self.device_id.clone();
        let http = cx.http_client();

        self.busy = true;
        self.error = None;
        cx.notify();

        self._quick_connect = cx.spawn(async move |this, cx| {
            let initiated =
                jellyfin::quick_connect_initiate(http.clone(), &server_input, &device_id).await;
            let (server, secret, code) = match initiated {
                Ok(v) => v,
                Err(message) => {
                    this.update(cx, |this, cx| {
                        this.busy = false;
                        this.error = Some(message.into());
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            let shown = this.update(cx, |this, cx| {
                this.busy = false;
                this.quick_connect = Some(QuickConnect { code });
                cx.notify();
            });
            if shown.is_err() {
                return;
            }
            loop {
                cx.background_executor()
                    .timer(QUICK_CONNECT_POLL_EVERY)
                    .await;
                match jellyfin::quick_connect_poll(http.clone(), &server, &secret).await {
                    Ok(true) => break,
                    Ok(false) => continue,
                    Err(message) => {
                        this.update(cx, |this, cx| {
                            this.quick_connect = None;
                            this.error = Some(message.into());
                            cx.notify();
                        })
                        .ok();
                        return;
                    }
                }
            }
            let result = jellyfin::quick_connect_login(http, &server, &secret, &device_id).await;
            this.update(cx, |this, cx| {
                this.quick_connect = None;
                match result {
                    Ok(session) => cx.emit(LoggedIn(session)),
                    Err(message) => this.error = Some(message.into()),
                }
                cx.notify();
            })
            .ok();
        });
    }

    /// Replacing the task cancels the in-flight request/poll, same as `load_users`.
    fn quick_connect_cancel(&mut self, cx: &mut Context<Self>) {
        self._quick_connect = Task::ready(());
        self.quick_connect = None;
        cx.notify();
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
        let busy = self.busy;
        let field = |label, input, cx: &App| text_field(label, input, busy, cx);

        centered(
            v_flex()
                .w(size::form_w())
                .gap(sp(6.))
                .child(brand_hero("Sign in to your Jellyfin server", cx))
                .child(panel(cx).map(|this| {
                    match &self.quick_connect {
                        Some(quick_connect) => this
                            .items_center()
                            .gap(sp(2.))
                            .child(caption(
                                "Enter this code on a device you're signed in on",
                                cx,
                            ))
                            .child(display(quick_connect.code.clone()))
                            .child(caption("Waiting for approval\u{2026}", cx))
                            .children(self.error.clone().map(|error| error_text(error, cx)))
                            .child(
                                outline_button("quick-connect-cancel", "Cancel", cx)
                                    .mt(sp(2.))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.quick_connect_cancel(cx)),
                                    ),
                            ),
                        None => this
                            .child(field("Server address", &self.server, cx))
                            .when(!self.users.is_empty(), |this| {
                                this.child(h_flex().flex_wrap().gap(sp(2.)).children(
                                    self.users.iter().enumerate().map(|(i, user)| {
                                        let picked = user.clone();
                                        outline_button(("user", i), user.name.clone(), cx)
                                            .sm()
                                            .disabled(self.busy)
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.pick_user(&picked, window, cx)
                                            }))
                                    }),
                                ))
                            })
                            .child(field("Username", &self.username, cx))
                            .child(field("Password", &self.password, cx))
                            .children(self.error.clone().map(|error| error_text(error, cx)))
                            .child(
                                primary_button("sign-in", "Sign in")
                                    .lg()
                                    .mt(sp(2.))
                                    .loading(self.busy)
                                    .disabled(self.busy)
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.submit(window, cx)),
                                    ),
                            )
                            .child(
                                outline_button("quick-connect", "Sign in with Quick Connect", cx)
                                    .lg()
                                    .disabled(self.busy)
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.quick_connect_start(cx)),
                                    ),
                            ),
                    }
                })),
        )
    }
}
