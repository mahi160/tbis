//! Watch stats (Settings > Stats), computed from the server's played items.
//! The server keeps only each item's last play date and play count, so the
//! 7/30-day totals and the weekly chart count each item once, at its last play.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::component::chart::BarChart;
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::jellyfin::{Api, Item, Kind};
use crate::status::{Status, inline_status};

const WEEKS: i64 = 12;
const TOP_SERIES: usize = 5;
const TICKS_PER_HOUR: f64 = 36_000_000_000.;

#[derive(Debug, Default, PartialEq)]
pub struct WatchStats {
    /// Hours.
    last_7_days: f64,
    last_30_days: f64,
    all_time: f64,
    movies: usize,
    episodes: usize,
    /// (Series name, Episodes finished), most first.
    top_series: Vec<(String, usize)>,
    /// (week start `MM-DD`, hours), oldest first, `WEEKS` long.
    weekly: Vec<(String, f64)>,
}

impl WatchStats {
    /// `today` in days since 1970-01-01 (UTC).
    fn compute(items: &[Item], today: i64) -> Self {
        let mut stats = Self::default();
        let first_week = today - WEEKS * 7 + 1;
        let mut weekly = vec![0.; WEEKS as usize];
        let mut series: HashMap<&str, usize> = HashMap::new();
        for item in items {
            let hours = item.run_time_ticks.unwrap_or(0) as f64 / TICKS_PER_HOUR;
            stats.all_time += hours * f64::from(item.user_data.play_count.max(1));
            match item.kind {
                Kind::Movie => stats.movies += 1,
                Kind::Episode => {
                    stats.episodes += 1;
                    if let Some(name) = &item.series_name {
                        *series.entry(name).or_default() += 1;
                    }
                }
                Kind::Series | Kind::Other => {}
            }
            let Some(day) = item
                .user_data
                .last_played_date
                .as_deref()
                .and_then(days_from_iso)
            else {
                continue;
            };
            let ago = today - day;
            if (0..7).contains(&ago) {
                stats.last_7_days += hours;
            }
            if (0..30).contains(&ago) {
                stats.last_30_days += hours;
            }
            if (first_week..=today).contains(&day) {
                weekly[((day - first_week) / 7) as usize] += hours;
            }
        }
        let mut top: Vec<(String, usize)> = series
            .into_iter()
            .map(|(name, n)| (name.to_string(), n))
            .collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        top.truncate(TOP_SERIES);
        stats.top_series = top;
        stats.weekly = weekly
            .into_iter()
            .enumerate()
            .map(|(week, hours)| {
                let (_, month, day) = civil_from_days(first_week + week as i64 * 7);
                (format!("{month:02}-{day:02}"), hours)
            })
            .collect();
        stats
    }
}

/// Days since 1970-01-01 for the `YYYY-MM-DD` start of an ISO 8601 timestamp.
fn days_from_iso(iso: &str) -> Option<i64> {
    let year: i64 = iso.get(0..4)?.parse().ok()?;
    let month: i64 = iso.get(5..7)?.parse().ok()?;
    let day: i64 = iso.get(8..10)?.parse().ok()?;
    // Howard Hinnant's days_from_civil
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// (year, month, day) for days since 1970-01-01; inverse of [`days_from_iso`].
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn today() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    (secs / 86_400) as i64
}

fn hours_label(hours: f64) -> String {
    if hours < 1. {
        format!("{} min", (hours * 60.).round())
    } else {
        format!("{hours:.1} h")
    }
}

pub struct StatsView {
    stats: Option<WatchStats>,
    error: Option<SharedString>,
    _load: Task<()>,
}

impl StatsView {
    pub fn new(api: Api, cx: &mut Context<Self>) -> Self {
        let _load = cx.spawn(async move |this, cx| {
            let result = api.played_items().await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(items) => this.stats = Some(WatchStats::compute(&items, today())),
                    Err(err) => this.error = Some(format!("Could not load stats: {err}").into()),
                }
                cx.notify();
            })
            .ok();
        });
        Self {
            stats: None,
            error: None,
            _load,
        }
    }
}

fn tile(label: &'static str, value: String, cx: &App) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_w(px(120.))
        .gap_1()
        .p_3()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .child(value),
        )
}

impl Render for StatsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = match (&self.error, &self.stats) {
            (Some(error), _) => Some(Status::Error(error.clone())),
            (None, None) => Some(Status::Loading),
            (None, Some(s)) if s.movies + s.episodes == 0 => {
                Some(Status::Empty("Nothing watched yet".into()))
            }
            _ => None,
        };
        let Some(stats) = self.stats.as_ref().filter(|_| status.is_none()) else {
            return v_flex().children(inline_status(status, cx));
        };
        let muted_fg = cx.theme().muted_foreground;
        let heading = |text: &'static str| {
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(text)
        };
        v_flex()
            .gap_5()
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_3()
                    .child(tile("Last 7 days", hours_label(stats.last_7_days), cx))
                    .child(tile("Last 30 days", hours_label(stats.last_30_days), cx))
                    .child(tile("All time", hours_label(stats.all_time), cx))
                    .child(tile("Movies", stats.movies.to_string(), cx))
                    .child(tile("Episodes", stats.episodes.to_string(), cx)),
            )
            .child(
                v_flex().gap_2().child(heading("Hours per week")).child(
                    div().h(px(180.)).child(
                        BarChart::new(stats.weekly.clone())
                            .id("weekly-hours")
                            .name("Hours")
                            .band(|(week, _)| SharedString::from(week.clone()))
                            .value(|(_, hours)| (hours * 10.).round() / 10.),
                    ),
                ),
            )
            .when(!stats.top_series.is_empty(), |this| {
                this.child(
                    v_flex()
                        .gap_2()
                        .child(heading("Most-watched Series"))
                        .children(stats.top_series.iter().map(|(name, count)| {
                            h_flex()
                                .justify_between()
                                .text_sm()
                                .child(div().truncate().child(name.clone()))
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .text_color(muted_fg)
                                        .child(format!("{count} episodes")),
                                )
                        })),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Item, Kind, WatchStats, civil_from_days, days_from_iso};

    #[test]
    fn dates_round_trip() {
        let day = days_from_iso("2024-03-01T12:00:00.0000000Z").unwrap();
        assert_eq!(civil_from_days(day), (2024, 3, 1));
        assert_eq!(days_from_iso("1970-01-01"), Some(0));
    }

    #[test]
    fn totals_counts_and_weeks() {
        let today = days_from_iso("2024-03-31").unwrap();
        let played = |kind, series: Option<&str>, hours: i64, count, date: &str| {
            let mut item = Item::default();
            item.kind = kind;
            item.series_name = series.map(Into::into);
            item.run_time_ticks = Some(hours * 36_000_000_000);
            item.user_data.play_count = count;
            item.user_data.last_played_date = Some(date.into());
            item
        };
        let items = [
            played(Kind::Movie, None, 2, 2, "2024-03-30"),
            played(Kind::Episode, Some("B"), 1, 1, "2024-03-20"),
            played(Kind::Episode, Some("A"), 1, 1, "2023-01-01"),
            played(Kind::Episode, Some("B"), 1, 1, "2023-01-01"),
        ];
        let stats = WatchStats::compute(&items, today);
        assert_eq!(stats.all_time, 7.);
        assert_eq!(stats.last_7_days, 2.);
        assert_eq!(stats.last_30_days, 3.);
        assert_eq!((stats.movies, stats.episodes), (1, 3));
        assert_eq!(
            stats.top_series,
            [("B".to_string(), 2), ("A".to_string(), 1)]
        );
        assert_eq!(stats.weekly.len(), 12);
        assert_eq!(stats.weekly[11].1, 2.);
        assert_eq!(stats.weekly[10].1, 1.);
    }
}
