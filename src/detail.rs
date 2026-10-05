//! Shared layout for Movie and Series detail pages (`movie.rs`, `series.rs`): both are
//! a Back button, poster, and a header body, over a scrollable page with the same padding.

use gpui_kit::*;

pub use crate::components::detail::*;
use crate::jellyfin::{Api, UserData};

const TICKS_PER_MINUTE: i64 = 600_000_000;

/// Ticks rounded to the nearest minute; shared rounding for every runtime display.
pub fn minutes(ticks: i64) -> i64 {
    (ticks + TICKS_PER_MINUTE / 2) / TICKS_PER_MINUTE
}

/// `1h 23m`, rounded to the nearest minute.
pub fn runtime_label(ticks: i64) -> String {
    let minutes = minutes(ticks);
    format!("{}h {:02}m", minutes / 60, minutes % 60)
}

/// Watched/Favorite flag a detail page can flip on one of its items.
#[derive(Clone, Copy)]
pub enum Toggle {
    Played,
    Favorite,
}

/// Detail page holding items whose `UserData` a `Toggle` flips.
pub trait UserDataView: Sized + 'static {
    fn user_data(&mut self, id: &str) -> Option<&mut UserData>;
    fn set_error(&mut self, error: SharedString);
    /// Server accepted the change; e.g. Series detail reloads its Episodes.
    fn saved(&mut self, _id: &str, _toggle: Toggle, _cx: &mut Context<Self>) {}
}

/// Flips `toggle` on item `id` now, then tells the server; on failure restores the
/// previous state and reports the error.
pub fn toggle<V: UserDataView>(
    view: &mut V,
    api: &Api,
    id: &str,
    toggle: Toggle,
    cx: &mut Context<V>,
) {
    let Some(data) = view.user_data(id) else {
        return;
    };
    let before = data.clone();
    let value = match toggle {
        Toggle::Played => {
            data.played = !data.played;
            // server resets the resume position both ways
            data.playback_position_ticks = 0;
            data.played_percentage = None;
            data.played
        }
        Toggle::Favorite => {
            data.is_favorite = !data.is_favorite;
            data.is_favorite
        }
    };
    cx.notify();
    let (api, id) = (api.clone(), id.to_string());
    cx.spawn(async move |this, cx| {
        let result = match toggle {
            Toggle::Played => api.set_played(&id, value).await,
            Toggle::Favorite => api.set_favorite(&id, value).await,
        };
        this.update(cx, |view, cx| {
            match result {
                Ok(()) => view.saved(&id, toggle, cx),
                Err(err) => {
                    if let Some(data) = view.user_data(&id) {
                        *data = before;
                    }
                    let what = match toggle {
                        Toggle::Played => "watched state",
                        Toggle::Favorite => "favorite",
                    };
                    view.set_error(format!("Could not update {what}: {err}").into());
                }
            }
            cx.notify();
        })
        .ok();
    })
    .detach();
}
