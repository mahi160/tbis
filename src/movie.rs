use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, IconName, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::card::{self, Play};
use crate::jellyfin::{Api, Item};

const PAD: f32 = 24.;
const TICKS_PER_MINUTE: i64 = 600_000_000;

pub struct Back;

/// Movie detail: poster, title, meta, overview, and a Play/Resume button.
pub struct MovieView {
    api: Api,
    movie: Item,
    loading: bool,
    error: Option<SharedString>,
    _load: Task<()>,
}

impl EventEmitter<Play> for MovieView {}
impl EventEmitter<Back> for MovieView {}

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
        let ticks = self.movie.run_time_ticks?;
        let minutes = (ticks + TICKS_PER_MINUTE / 2) / TICKS_PER_MINUTE;
        Some(format!("{}h {:02}m", minutes / 60, minutes % 60))
    }
}

impl Render for MovieView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_fg = cx.theme().muted_foreground;
        let resumed = self.movie.user_data.playback_position_ticks > 0;
        let runtime = self.runtime_label();
        let movie = self.movie.clone();

        let header = h_flex()
            .gap_5()
            .items_start()
            .child(
                Button::new("movie-back")
                    .ghost()
                    .icon(IconName::ArrowLeft)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Back))),
            )
            .child(
                div()
                    .w(px(160.))
                    .h(px(240.))
                    .flex_shrink_0()
                    .rounded_md()
                    .overflow_hidden()
                    .child(card::image(
                        self.api.poster_url(&self.movie),
                        self.movie.name.clone().into(),
                        cx,
                    )),
            )
            .child(
                v_flex()
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
                                cx.emit(Play(movie.clone()))
                            })),
                    ),
            );

        let status = self
            .error
            .clone()
            .map(|error| div().text_sm().text_color(cx.theme().danger).child(error));

        v_flex()
            .id("movie-detail")
            .size_full()
            .overflow_y_scroll()
            .p(px(PAD))
            .gap_6()
            .child(header)
            .children(status)
    }
}
