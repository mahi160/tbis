use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, IconName, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::detail::{self, PAD, Toggle, UserDataView};
use crate::jellyfin::{Api, Item, Kind, UserData, episode_code};
use crate::nav::Nav;
use crate::status::{Status, inline_status};

/// Movie or Episode detail: art, title, meta, overview, and a Play/Resume button.
/// An Episode shows its still instead of a poster, under its Series and code.
pub struct MovieView {
    api: Api,
    movie: Item,
    loading: bool,
    error: Option<SharedString>,
    _load: Task<()>,
}

impl EventEmitter<Nav> for MovieView {}

impl UserDataView for MovieView {
    fn user_data(&mut self, id: &str) -> Option<&mut UserData> {
        (self.movie.id == id).then_some(&mut self.movie.user_data)
    }

    fn set_error(&mut self, error: SharedString) {
        self.error = Some(error);
    }
}

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

    /// `Series · S01E03 · Aired 2024-05-01` above an Episode's title.
    fn episode_kicker(&self, cx: &App) -> impl IntoElement {
        let code = match (self.movie.parent_index_number, self.movie.index_number) {
            (Some(season), Some(episode)) => Some(episode_code(season, episode)),
            _ => None,
        };
        let aired = self
            .movie
            .premiere_date
            .as_deref()
            .and_then(|date| date.get(..10))
            .map(|day| format!("Aired {day}"));
        let parts: Vec<String> = [self.movie.series_name.clone(), code, aired]
            .into_iter()
            .flatten()
            .collect();
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(parts.join(" \u{b7} "))
    }

    fn runtime_label(&self) -> Option<String> {
        Some(detail::runtime_label(self.movie.run_time_ticks?))
    }
}

impl Render for MovieView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let resumed = self.movie.user_data.playback_position_ticks > 0;
        let runtime = self.runtime_label();
        let movie = self.movie.clone();

        let episode = self.movie.kind == Kind::Episode;
        let back = Box::new(cx.listener(|_, _, _, cx| cx.emit(Nav::Back)));
        let body = v_flex()
            .flex_1()
            .min_w_0()
            .gap_3()
            .when(episode, |this| this.child(self.episode_kicker(cx)))
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child(self.movie.display_name().to_string()),
            )
            .children(detail::meta_line(&self.movie, runtime, cx))
            .children(detail::genres_line(&self.movie, cx))
            .children(detail::media_tags(&self.movie))
            .children(self.movie.overview.clone().map(|o| {
                div()
                    .text_sm()
                    .max_w(px(560.))
                    .line_clamp(6)
                    .text_ellipsis()
                    .child(o)
            }))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("movie-play")
                            .primary()
                            .icon(IconName::Play)
                            .label(if resumed { "Resume" } else { "Play" })
                            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                cx.emit(Nav::Play(movie.clone()))
                            })),
                    )
                    .children([Toggle::Played, Toggle::Favorite].map(|toggle| {
                        let id = self.movie.id.clone();
                        detail::toggle_button(
                            ("movie-toggle", toggle as usize),
                            toggle,
                            &self.movie.user_data,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let api = this.api.clone();
                            detail::toggle(this, &api, &id, toggle, cx)
                        }))
                    })),
            );
        let image = if episode {
            (self.api.wide_image_url(&self.movie), px(320.), px(180.))
        } else {
            (self.api.poster_url(&self.movie), px(160.), px(240.))
        };
        let header = detail::detail_header(image, back, body, cx);

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
            .relative()
            .overflow_y_scroll()
            .p(px(PAD))
            .gap_6()
            .children(detail::backdrop(&self.api, &self.movie, cx))
            .child(header)
            .children(inline_status(status, cx))
            .children(detail::cast_row(&self.api, &self.movie, cx))
    }
}
