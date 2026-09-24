use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, IconName, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::detail::{self, PAD};
use crate::jellyfin::{Api, Item};
use crate::nav::Nav;
use crate::status::{Status, inline_status};

/// Movie detail: poster, title, meta, overview, and a Play/Resume button.
pub struct MovieView {
    api: Api,
    movie: Item,
    loading: bool,
    error: Option<SharedString>,
    _load: Task<()>,
}

impl EventEmitter<Nav> for MovieView {}

impl MovieView {
    pub fn new(api: Api, movie: Item, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            api,
            movie,
            loading: true,
            error: None,
            _load: Task::ready(()),
        };
        this.refresh(cx);
        this
    }

    /// Reloads full details (overview, runtime); keeps showing the current item until they arrive.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let api = self.api.clone();
        let id = self.movie.id.clone();
        self.loading = true;
        cx.notify();
        self._load = cx.spawn(async move |this, cx| {
            let result = api.item(&id).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(details) => {
                        this.movie = details;
                        this.error = None;
                    }
                    Err(err) => this.error = Some(format!("Could not load movie: {err}").into()),
                }
                cx.notify();
            })
            .ok();
        });
    }

    fn runtime_label(&self) -> Option<String> {
        Some(detail::runtime_label(self.movie.run_time_ticks?))
    }
}

impl Render for MovieView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_fg = cx.theme().muted_foreground;
        let resumed = self.movie.user_data.playback_position_ticks > 0;
        let runtime = self.runtime_label();
        let movie = self.movie.clone();

        let back = Box::new(cx.listener(|_, _, _, cx| cx.emit(Nav::Back)));
        let body = v_flex()
            .min_w_0()
            .gap_3()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.movie.name.clone()),
            )
            .when(
                self.movie.production_year.is_some() || runtime.is_some(),
                |this| {
                    this.child(
                        h_flex()
                            .gap_3()
                            .text_sm()
                            .text_color(muted_fg)
                            .children(self.movie.production_year.map(|y| y.to_string()))
                            .children(runtime),
                    )
                },
            )
            .children(
                self.movie
                    .overview
                    .clone()
                    .map(|o| div().text_sm().max_w(px(560.)).line_clamp(6).child(o)),
            )
            .child(
                Button::new("movie-play")
                    .primary()
                    .icon(IconName::Play)
                    .label(if resumed { "Resume" } else { "Play" })
                    .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                        cx.emit(Nav::Play(movie.clone()))
                    })),
            );
        let header =
            detail::detail_header(&self.api, &self.movie, (px(160.), px(240.)), back, body, cx);

        let status = if let Some(error) = self.error.clone() {
            Some(Status::Error(error))
        } else if self.loading {
            Some(Status::Loading)
        } else {
            None
        };

        v_flex()
            .id("movie-detail")
            .size_full()
            .overflow_y_scroll()
            .p(px(PAD))
            .gap_6()
            .child(header)
            .children(inline_status(status, cx))
    }
}
