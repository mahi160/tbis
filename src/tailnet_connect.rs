use gpui_kit::*;

use crate::components::tailnet::{self, Authorization};
use crate::connection::{self, Service};

pub enum ConnectionEvent {
    Connected,
    Cancelled,
}

pub struct TailnetConnectView {
    service: Service,
    authorization: Option<Authorization>,
    error: Option<SharedString>,
    task: Task<()>,
}

impl EventEmitter<ConnectionEvent> for TailnetConnectView {}

impl TailnetConnectView {
    pub fn new(service: Service, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            service,
            authorization: None,
            error: None,
            task: Task::ready(()),
        };
        view.connect(cx);
        view
    }

    fn connect(&mut self, cx: &mut Context<Self>) {
        self.authorization = None;
        self.error = None;
        let service = self.service.clone();
        self.task = cx.spawn(async move |this, cx| {
            let executor = cx.background_executor().clone();
            let result = connection::authorize(&service, &executor, |url| {
                this.update(cx, |this, cx| {
                    let url = url.filter(|url| {
                        url::Url::parse(url)
                            .is_ok_and(|url| url.scheme() == "https" && url.host_str().is_some())
                    });
                    if this.authorization.as_ref().map(|auth| auth.url()) != url.as_deref() {
                        this.authorization = url.map(Authorization::new);
                        cx.notify();
                    }
                })
                .ok();
            })
            .await;
            this.update(cx, |this, cx| {
                this.authorization = None;
                match result {
                    Ok(()) => cx.emit(ConnectionEvent::Connected),
                    Err(err) => this.error = Some(err.to_string().into()),
                }
                cx.notify();
            })
            .ok();
        });
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.task = Task::ready(());
        cx.emit(ConnectionEvent::Cancelled);
    }
}

impl Render for TailnetConnectView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        tailnet::connect_screen(
            self.authorization.as_ref(),
            self.error.clone(),
            cx.listener(|this, _, _, cx| this.connect(cx)),
            cx.listener(|this, _, _, cx| this.cancel(cx)),
            cx,
        )
    }
}
