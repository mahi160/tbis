use gpui_kit::component::input::{InputEvent, InputState};
use std::time::Duration;

use gpui_kit::component::{Disableable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::components::button::{VegaSize as _, outline_button, primary_button};
use crate::components::input::text_field;
use crate::components::layout::centered_scroll;
use crate::components::logo::brand_hero;
use crate::components::panel::panel;
use crate::components::tailnet;
use crate::components::typography::{caption, error_text, title};
use crate::config::{Config, TailscaleOptions};
use crate::connection::{self, Route, Service};
use crate::design::{gap, size};
use crate::jellyfin::{self, PublicUser, Session};
use crate::tailnet_connect::{ConnectionEvent, TailnetConnectView};

/// Wait after typing in Server before asking it for users.
const USERS_DEBOUNCE: Duration = Duration::from_millis(400);
/// How often to ask the server whether the Quick Connect code was approved.
const QUICK_CONNECT_POLL_EVERY: Duration = Duration::from_secs(2);

pub enum LoginEvent {
    LoggedIn(Session, Route),
    OptionsChanged(TailscaleOptions),
}

/// A Quick Connect code shown to the user while `LoginView::_quick_connect` polls
/// for approval.
struct QuickConnect {
    code: String,
}

pub struct LoginView {
    server: Entity<InputState>,
    address: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    device_id: String,
    options: TailscaleOptions,
    saved_options: TailscaleOptions,
    service: Service,
    route: Option<Route>,
    restore: Option<Session>,
    connection: Option<(Entity<TailnetConnectView>, Subscription)>,
    connecting: bool,
    error: Option<SharedString>,
    route_error: Option<SharedString>,
    busy: bool,
    users: Vec<PublicUser>,
    quick_connect: Option<QuickConnect>,
    _users: Task<()>,
    _login: Task<()>,
    _quick_connect: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<LoginEvent> for LoginView {}

impl LoginView {
    pub fn new(
        config: &Config,
        service: Service,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let server = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("192.168.1.5:8096");
            if let Some(session) = &config.session {
                input.set_value(session.server.clone(), window, cx);
            }
            input
        });
        let address = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("jellyfin or 100.64.0.1:8096");
            input.set_value(config.tailscale.address.clone(), window, cx);
            input
        });
        let username = cx.new(|cx| InputState::new(window, cx));
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Optional")
                .masked(true)
        });
        let mut subscriptions: Vec<Subscription> = [&server, &address, &username, &password]
            .into_iter()
            .map(|input| {
                cx.subscribe_in(input, window, |this, _, event, window, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        this.submit(window, cx);
                    }
                })
            })
            .collect();
        subscriptions.push(cx.subscribe(&server, |this, _, event, cx| {
            if let InputEvent::Change = event {
                this.load_users(false, cx);
            }
        }));
        subscriptions.push(cx.subscribe(&address, |this, _, event, cx| {
            if let InputEvent::Change = event {
                // Never send a saved token to a newly edited tailnet address.
                this.restore = None;
                this.load_users(false, cx);
            }
        }));
        server.update(cx, |input, cx| input.focus(window, cx));
        let mut view = Self {
            server,
            address,
            username,
            password,
            device_id: config.device_id.clone(),
            options: config.tailscale.clone(),
            saved_options: config.tailscale.clone(),
            service,
            route: None,
            restore: config.session.clone(),
            connection: None,
            connecting: false,
            error: None,
            route_error: None,
            busy: false,
            users: Vec::new(),
            quick_connect: None,
            _users: Task::ready(()),
            _login: Task::ready(()),
            _quick_connect: Task::ready(()),
            _subscriptions: subscriptions,
        };
        if config.session.is_some() {
            view.load_users(true, cx);
        }
        view
    }

    /// Shown after the server rejected the saved token: server kept, expiry explained.
    pub fn expired(&mut self, server: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.server.update(cx, |input, cx| {
            input.set_value(server.to_string(), window, cx)
        });
        self.error = Some(jellyfin::SessionExpired.to_string().into());
        cx.notify();
    }

    /// Writes the options to config once they differ from what was last saved.
    fn emit_options(&mut self, cx: &mut Context<Self>) {
        if self.saved_options != self.options {
            self.saved_options = self.options.clone();
            cx.emit(LoginEvent::OptionsChanged(self.options.clone()));
        }
    }

    /// Silent about the user list: failures just mean no picker. Route failures show under
    /// the fields and Sign in reports real errors. Only `interactive` calls may open the
    /// Tailscale QR screen; typing never does.
    fn load_users(&mut self, interactive: bool, cx: &mut Context<Self>) {
        let server = self.server.read(cx).value().trim().to_string();
        self.options.address = self.address.read(cx).value().trim().to_string();
        if self.restore.as_ref().is_some_and(|session| {
            jellyfin::normalize_server(&session.server) != jellyfin::normalize_server(&server)
        }) {
            // Never send a saved token to an address edited during session restoration.
            self.restore = None;
        }
        self._login = Task::ready(());
        self._quick_connect = Task::ready(());
        self.quick_connect = None;
        self.busy = false;
        self.route = None;
        self.users.clear();
        self.connecting = !server.is_empty();
        cx.notify();
        if !self.connecting {
            // Options persist even without a server to look up.
            self._users = cx.spawn(async move |this, cx| {
                cx.background_executor().timer(USERS_DEBOUNCE).await;
                this.update(cx, |this, cx| this.emit_options(cx)).ok();
            });
            return;
        }
        let debounce = self.restore.is_none();
        let http = cx.http_client();
        let service = self.service.clone();
        let options = self.options.clone();
        // replacing task cancels older lookup
        self._users = cx.spawn(async move |this, cx| {
            let executor = cx.background_executor().clone();
            if debounce {
                executor.timer(USERS_DEBOUNCE).await;
            }
            this.update(cx, |this, cx| this.emit_options(cx)).ok();
            let route =
                connection::resolve(&service, http.clone(), &executor, &server, &options).await;
            let route = match route {
                Ok(route) => route,
                Err(err) => {
                    this.update(cx, |this, cx| {
                        this.connecting = false;
                        if !err.is::<connection::NeedsAuthorization>() {
                            this.route_error = Some(err.to_string().into());
                        } else if interactive {
                            this.authorize_tailnet(cx);
                        } else {
                            this.route_error = Some(
                                "Tailscale needs authorization. Choose Sign in to connect.".into(),
                            );
                        }
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            let restored = this
                .update(cx, |this, cx| {
                    this.route = Some(route.clone());
                    this.connecting = false;
                    this.route_error = None;
                    cx.notify();
                    if let Some(session) = this.restore.take() {
                        cx.emit(LoginEvent::LoggedIn(session, route.clone()));
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(true);
            if restored {
                return;
            }
            let users = jellyfin::public_users(http, &route.server)
                .await
                .unwrap_or_default();
            this.update(cx, |this, cx| {
                this.connecting = false;
                this.users = users;
                cx.notify();
            })
            .ok();
        });
    }

    fn set_tailscale(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.options.enabled = enabled;
        // Never send a saved token to a newly chosen tailnet destination.
        self.restore = None;
        self.load_users(true, cx);
        self.emit_options(cx);
    }

    fn authorize_tailnet(&mut self, cx: &mut Context<Self>) {
        self._users = Task::ready(());
        self._login = Task::ready(());
        self._quick_connect = Task::ready(());
        self.quick_connect = None;
        self.busy = false;
        self.connecting = false;
        let service = self.service.clone();
        let view = cx.new(|cx| TailnetConnectView::new(service, cx));
        let subscription = cx.subscribe(&view, |this, _, event, cx| {
            this.connection = None;
            if let ConnectionEvent::Connected = event {
                this.load_users(true, cx);
            }
            cx.notify();
        });
        self.connection = Some((view, subscription));
        cx.notify();
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
        if self.busy || self.connecting || self.quick_connect.is_some() {
            return;
        }
        let Some(route) = self.route.clone() else {
            self.load_users(true, cx);
            return;
        };
        let saved_server = jellyfin::normalize_server(&self.server.read(cx).value());
        let device_id = self.device_id.clone();
        let http = cx.http_client();
        self.busy = true;
        self.error = None;
        cx.notify();
        self._quick_connect = cx.spawn(async move |this, cx| {
            let initiated =
                jellyfin::quick_connect_initiate(http.clone(), &route.server, &device_id).await;
            let (server, secret, code) = match initiated {
                Ok(value) => value,
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
            if this
                .update(cx, |this, cx| {
                    this.busy = false;
                    this.quick_connect = Some(QuickConnect { code });
                    cx.notify();
                })
                .is_err()
            {
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
                    Ok(mut session) => {
                        session.server = saved_server;
                        cx.emit(LoginEvent::LoggedIn(session, route));
                    }
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
        if self.busy || self.connecting {
            return;
        }
        let Some(route) = self.route.clone() else {
            self.load_users(true, cx);
            return;
        };
        let saved_server = jellyfin::normalize_server(&self.server.read(cx).value());
        let username = self.username.read(cx).value().to_string();
        let password = self.password.read(cx).value().to_string();
        let device_id = self.device_id.clone();
        let http = cx.http_client();
        self.busy = true;
        self.error = None;
        cx.notify();
        self._login = cx.spawn(async move |this, cx| {
            let result =
                jellyfin::login(http, &route.server, &username, &password, &device_id).await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(mut session) => {
                        session.server = saved_server;
                        cx.emit(LoginEvent::LoggedIn(session, route));
                    }
                    Err(message) => this.error = Some(message.into()),
                }
                cx.notify();
            })
            .ok();
        });
    }
}

impl Render for LoginView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some((view, _)) = &self.connection {
            return view.clone().into_any_element();
        }
        let login = cx.entity();
        let field = |label, input, cx: &App| text_field(label, input, self.busy, cx);
        centered_scroll(
            "login",
            v_flex()
                .w(size::form_w())
                .gap(gap::group())
                .child(brand_hero("Sign in to your Jellyfin server", cx))
                .child(panel(cx).map(|view| {
                    match &self.quick_connect {
                        Some(quick_connect) => view
                            .items_center()
                            .gap(gap::item())
                            .child(caption(
                                "Enter this code on a device you're signed in on",
                                cx,
                            ))
                            .child(title(quick_connect.code.clone()))
                            .child(caption("Waiting for approval\u{2026}", cx))
                            .children(self.error.clone().map(|error| error_text(error, cx)))
                            .child(
                                outline_button("quick-connect-cancel", "Cancel", cx)
                                    .mt(gap::item())
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.quick_connect_cancel(cx)),
                                    ),
                            ),
                        None => view
                            .child(field("Server address", &self.server, cx))
                            .child(tailnet::use_tailscale_toggle(
                                self.options.enabled,
                                self.busy,
                                move |on, _, cx| {
                                    login.update(cx, |this, cx| this.set_tailscale(on, cx))
                                },
                            ))
                            .when(self.options.enabled, |view| {
                                view.child(field("Tailnet address", &self.address, cx))
                            })
                            .when(self.connecting, |view| {
                                view.child(tailnet::server_connection_pending(
                                    cx.listener(|this, _, _, cx| {
                                        this._users = Task::ready(());
                                        this.connecting = false;
                                        this.restore = None;
                                        cx.notify();
                                    }),
                                    cx,
                                ))
                            })
                            .children(self.route_error.clone().map(|error| error_text(error, cx)))
                            .when(!self.users.is_empty(), |view| {
                                view.child(h_flex().flex_wrap().gap(gap::item()).children(
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
                                    .mt(gap::item())
                                    .loading(self.busy)
                                    .disabled(self.busy || self.connecting)
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.submit(window, cx)),
                                    ),
                            )
                            .child(
                                outline_button("quick-connect", "Sign in with Quick Connect", cx)
                                    .lg()
                                    .disabled(self.busy || self.connecting)
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.quick_connect_start(cx)),
                                    ),
                            ),
                    }
                })),
        )
        .into_any_element()
    }
}
