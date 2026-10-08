use gpui_kit::component::button::Button;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use crate::components::backdrop::backdrop;
use crate::components::episode::{EpisodeRow, episode_row};
use crate::components::layout::scroll_page;
use crate::components::status::{Status, inline_status};
use crate::components::tabs::scrolling_tabs;
use crate::components::toggle_button::toggle_button;
use crate::components::typography::{paragraph, title};
use crate::config::HideSpoilers;
use crate::design::{gap, size};
use crate::detail::{self, Toggle, UserDataView};
use crate::jellyfin::{Api, Item, UserData, blurred};
use crate::nav::Nav;

/// Series detail: header, season tabs, and the Episodes of the selected season.
pub struct SeriesView {
    api: Api,
    series: Item,
    seasons: Vec<Item>,
    selected: usize,
    episodes: Vec<Item>,
    loading: bool,
    error: Option<SharedString>,
    /// Kept by the view so Back from an Episode page lands where the user was.
    scroll: ScrollHandle,
    _load: Task<()>,
}

impl EventEmitter<Nav> for SeriesView {}

impl UserDataView for SeriesView {
    fn user_data(&mut self, id: &str) -> Option<&mut UserData> {
        if self.series.id == id {
            return Some(&mut self.series.user_data);
        }
        self.episodes
            .iter_mut()
            .find(|e| e.id == id)
            .map(|e| &mut e.user_data)
    }

    fn set_error(&mut self, error: SharedString) {
        self.error = Some(error);
    }

    /// Whole-Series watched flips every Episode server-side; reload them.
    fn saved(&mut self, id: &str, toggle: Toggle, cx: &mut Context<Self>) {
        if matches!(toggle, Toggle::Played) && id == self.series.id {
            self.load_episodes(cx);
        }
    }
}

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
            scroll: ScrollHandle::new(),
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

    /// `3 seasons`, from the loaded season list.
    fn seasons_label(&self) -> Option<String> {
        match self.seasons.len() {
            0 => None,
            1 => Some("1 season".into()),
            n => Some(format!("{n} seasons")),
        }
    }

    fn toggle_button(
        &self,
        item: &Item,
        element_id: ElementId,
        toggle: Toggle,
        cx: &mut Context<Self>,
    ) -> Button {
        let id = item.id.clone();
        toggle_button(element_id, toggle, &item.user_data, cx).on_click(cx.listener(
            move |this, _, _, cx| {
                // gpui-kit Button doesn't stop the click; Episode row would play
                cx.stop_propagation();
                let api = this.api.clone();
                detail::toggle(this, &api, &id, toggle, cx)
            },
        ))
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let back = Box::new(cx.listener(|_, _, _, cx| cx.emit(Nav::Back)));
        let body = detail::header_body()
            .gap(gap::item())
            .child(title(self.series.name.clone()))
            .children(detail::meta_line(&self.series, self.seasons_label(), cx))
            .children(detail::genres_line(&self.series, cx))
            .children(self.series.overview.clone().map(|o| paragraph(o, 4)))
            .child(
                h_flex()
                    .gap(gap::item())
                    .children([Toggle::Played, Toggle::Favorite].map(|toggle| {
                        self.toggle_button(
                            &self.series,
                            ("series-toggle", toggle as usize).into(),
                            toggle,
                            cx,
                        )
                    })),
            );
        detail::detail_header(
            (
                self.api.poster_url(&self.series),
                size::header_poster_sm_w(),
                size::header_poster_sm_h(),
            ),
            back,
            body,
            cx,
        )
    }

    fn render_episode(&self, episode: &Item, cx: &mut Context<Self>) -> AnyElement {
        let open = {
            let episode = episode.clone();
            cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Nav::Open(episode.clone())))
        };
        let play = {
            let episode = episode.clone();
            cx.listener(move |_, _: &ClickEvent, _, cx| {
                cx.stop_propagation(); // row itself opens the Episode page
                cx.emit(Nav::Play(episode.clone()))
            })
        };
        let progress = episode.user_data.played_percentage.filter(|p| *p > 0.);
        let number = episode
            .index_number
            .map(|n| format!("{n}. "))
            .unwrap_or_default();
        let runtime = episode
            .run_time_ticks
            .map(|t| format!("{} min", detail::minutes(t)));
        let spoiler = cx.global::<HideSpoilers>().0 && !episode.user_data.played;
        let still = self
            .api
            .poster_url(episode)
            .map(|url| if spoiler { blurred(&url) } else { url });

        let row = EpisodeRow {
            id: episode.id.clone().into(),
            still,
            title: format!("{number}{}", episode.display_name()),
            runtime,
            rating: episode.community_rating,
            overview: episode.overview.clone().filter(|_| !spoiler),
            progress,
            played: episode.user_data.played,
        };
        let toggle = self.toggle_button(
            episode,
            ElementId::Name(format!("episode-played-{}", episode.id).into()),
            Toggle::Played,
            cx,
        );
        episode_row(row, Box::new(open), Box::new(play), toggle, cx)
    }
}

impl Render for SeriesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let seasons = (!self.seasons.is_empty()).then(|| {
            scrolling_tabs(
                "seasons",
                self.seasons.iter().map(Self::season_label),
                self.selected,
                cx.listener(|this, index: &usize, _, cx| this.select_season(*index, cx)),
            )
        });

        let none = self.episodes.is_empty();
        let status = Status::pick(
            self.error.clone(),
            self.loading && none,
            none.then(|| "No episodes".into()),
        );

        let mut episodes = Vec::with_capacity(self.episodes.len());
        for episode in &self.episodes {
            episodes.push(self.render_episode(episode, cx));
        }

        scroll_page("series-detail")
            .relative()
            .track_scroll(&self.scroll)
            .gap(gap::group())
            .children(
                self.api
                    .backdrop_url(&self.series)
                    .map(|url| backdrop(url, cx)),
            )
            .child(self.render_header(cx))
            .children(detail::cast_row(&self.api, &self.series, cx))
            .children(seasons)
            .children(inline_status(status, cx))
            .child(v_flex().gap(gap::tight()).children(episodes))
    }
}
