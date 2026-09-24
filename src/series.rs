use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::{ActiveTheme as _, IconName, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::card::{self, Play};
use crate::jellyfin::{Api, Item};

const PAD: f32 = 24.;
const TICKS_PER_MINUTE: i64 = 600_000_000;

pub struct Back;

/// Series detail: header, season tabs, and the Episodes of the selected season.
pub struct SeriesView {
    api: Api,
    series: Item,
    seasons: Vec<Item>,
    selected: usize,
    episodes: Vec<Item>,
    loading: bool,
    error: Option<SharedString>,
    _load: Task<()>,
}

impl EventEmitter<Play> for SeriesView {}
impl EventEmitter<Back> for SeriesView {}

impl SeriesView {
    pub fn new(api: Api, series: Item, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            api,
            series,
            seasons: Vec::new(),
            selected: 0,
            episodes: Vec::new(),
            loading: true,
            error: None,
            _load: Task::ready(()),
        };
        this.load_series(cx);
        this
    }

    /// Details, seasons, and Next Up together; opens on Next Up's season, else the first.
    fn load_series(&mut self, cx: &mut Context<Self>) {
        let api = self.api.clone();
        let id = self.series.id.clone();
        self._load = cx.spawn(async move |this, cx| {
            let (details, seasons, next_up) =
                futures::join!(api.item(&id), api.seasons(&id), api.next_up(&id));
            this.update(cx, |this, cx| match seasons {
                Ok(seasons) => {
                    if let Ok(details) = details {
                        this.series = details;
                    }
                    let next_season = next_up.ok().flatten().and_then(|e| e.season_id);
                    this.selected = seasons
                        .iter()
                        .position(|s| Some(&s.id) == next_season.as_ref())
                        .unwrap_or(0);
                    this.seasons = seasons;
                    this.load_episodes(cx);
                }
                Err(err) => {
                    this.loading = false;
                    this.error = Some(format!("Could not load series: {err}").into());
                    cx.notify();
                }
            })
            .ok();
        });
    }

    /// Reloads the selected season; keeps the current list until fresh data arrives.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.seasons.is_empty() {
            self.load_series(cx);
        } else {
            self.load_episodes(cx);
        }
    }

    fn select_season(&mut self, index: usize, cx: &mut Context<Self>) {
        if index == self.selected || index >= self.seasons.len() {
            return;
        }
        self.selected = index;
        self.episodes.clear();
        self.load_episodes(cx);
    }

    fn load_episodes(&mut self, cx: &mut Context<Self>) {
        let Some(season) = self.seasons.get(self.selected) else {
            self.loading = false;
            cx.notify();
            return;
        };
        let (api, series_id, season_id) =
            (self.api.clone(), self.series.id.clone(), season.id.clone());
        self.loading = true;
        self._load = cx.spawn(async move |this, cx| {
            let result = api.episodes(&series_id, &season_id).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(episodes) => {
                        this.episodes = episodes;
                        this.error = None;
                    }
                    Err(err) => this.error = Some(format!("Could not load episodes: {err}").into()),
                }
                cx.notify();
            })
            .ok();
        });
        cx.notify();
    }

    fn season_label(season: &Item) -> String {
        match season.index_number {
            Some(0) => "Specials".into(),
            _ => season.name.clone(),
        }
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_fg = cx.theme().muted_foreground;
        h_flex()
            .gap_5()
            .items_start()
            .child(
                Button::new("series-back")
                    .ghost()
                    .icon(IconName::ArrowLeft)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Back))),
            )
            .child(
                div()
                    .w(px(120.))
                    .h(px(180.))
                    .flex_shrink_0()
                    .rounded_md()
                    .overflow_hidden()
                    .child(card::image(
                        self.api.poster_url(&self.series),
                        self.series.name.clone().into(),
                        cx,
                    )),
            )
            .child(
                v_flex()
                    .min_w_0()
                    .gap_2()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.series.name.clone()),
                    )
                    .children(
                        self.series
                            .production_year
                            .map(|y| div().text_sm().text_color(muted_fg).child(y.to_string())),
                    )
                    .children(
                        self.series
                            .overview
                            .clone()
                            .map(|o| div().text_sm().line_clamp(4).child(o)),
                    ),
            )
    }

    fn render_episode(&self, episode: &Item, cx: &mut Context<Self>) -> AnyElement {
        let (muted, muted_fg) = (cx.theme().muted, cx.theme().muted_foreground);
        let play = {
            let episode = episode.clone();
            cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Play(episode.clone())))
        };
        let progress = episode.user_data.played_percentage.filter(|p| *p > 0.);
        let number = episode
            .index_number
            .map(|n| format!("{n}. "))
            .unwrap_or_default();
        let runtime = episode
            .run_time_ticks
            .map(|t| format!("{} min", (t + TICKS_PER_MINUTE / 2) / TICKS_PER_MINUTE));

        h_flex()
            .id(SharedString::from(episode.id.clone()))
            .gap_4()
            .p_2()
            .rounded_md()
            .cursor_pointer()
            .hover(|this| this.bg(muted))
            .on_click(play)
            .child(
                div()
                    .relative()
                    .w(px(192.))
                    .h(px(108.))
                    .flex_shrink_0()
                    .rounded_md()
                    .overflow_hidden()
                    .bg(muted)
                    .child(card::image(self.api.poster_url(episode), "".into(), cx))
                    .when_some(progress, |this, percent| {
                        this.child(card::progress_bar(percent, cx))
                    })
                    .when(episode.user_data.played, |this| {
                        this.child(card::check_badge(cx))
                    }),
            )
            .child(
                v_flex()
                    .min_w_0()
                    .gap_1()
                    .child(
                        div()
                            .truncate()
                            .child(format!("{number}{}", episode.display_name())),
                    )
                    .children(runtime.map(|r| div().text_sm().text_color(muted_fg).child(r)))
                    .children(
                        episode
                            .overview
                            .clone()
                            .map(|o| div().text_sm().text_color(muted_fg).line_clamp(2).child(o)),
                    ),
            )
            .into_any_element()
    }
}

impl Render for SeriesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_fg = cx.theme().muted_foreground;
        let seasons = (!self.seasons.is_empty()).then(|| {
            div().id("season-tabs").overflow_x_scroll().child(
                TabBar::new("seasons")
                    .underline()
                    .children(self.seasons.iter().map(Self::season_label))
                    .selected_index(self.selected)
                    .on_click(
                        cx.listener(|this, index: &usize, _, cx| this.select_season(*index, cx)),
                    ),
            )
        });

        let status = if let Some(error) = self.error.clone() {
            Some(div().text_color(cx.theme().danger).child(error))
        } else if self.episodes.is_empty() {
            Some(div().text_color(muted_fg).child(if self.loading {
                "Loading…"
            } else {
                "No episodes"
            }))
        } else {
            None
        };

        let mut episodes = Vec::with_capacity(self.episodes.len());
        for episode in &self.episodes {
            episodes.push(self.render_episode(episode, cx));
        }

        v_flex()
            .id("series-detail")
            .size_full()
            .overflow_y_scroll()
            .p(px(PAD))
            .gap_6()
            .child(self.render_header(cx))
            .children(seasons)
            .children(status)
            .child(v_flex().gap_1().children(episodes))
    }
}
