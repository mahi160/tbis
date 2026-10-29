//! Shared layout for Movie and Series detail pages (`movie.rs`, `series.rs`): both are
//! a Back button, poster, and a header body, over a scrollable page with the same padding.

use std::collections::HashMap;
use gpui_kit::*;

use crate::components::toggle_button::Toggle;
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

/// Detail page holding items whose `UserData` a `Toggle` flips.
#[derive(Default)]
pub struct UserDataMutations {
    tails: HashMap<String, futures::channel::oneshot::Receiver<()>>,
    versions: HashMap<(String, bool), u64>,
    refresh_versions: HashMap<(String, bool), u64>,
    confirmed: HashMap<(String, bool), UserData>,
    optimistic: HashMap<(String, bool), UserData>,
    pending: HashMap<(String, bool), usize>,
    watched_revision: u64,
    watched_pending: usize,
}

impl UserDataMutations {
    fn enqueue(
        &mut self,
        id: &str,
    ) -> (
        Option<futures::channel::oneshot::Receiver<()>>,
        futures::channel::oneshot::Sender<()>,
    ) {
        let previous = self.tails.remove(id);
        let (finished, tail) = futures::channel::oneshot::channel();
        self.tails.insert(id.to_string(), tail);
        (previous, finished)
    }

    fn begin(&mut self, id: &str, toggle: Toggle, before: &UserData) -> u64 {
        let key = (id.to_string(), matches!(toggle, Toggle::Favorite));
        if self.pending.get(&key).copied().unwrap_or_default() == 0 {
            self.confirmed.insert(key.clone(), before.clone());
        }
        *self.pending.entry(key.clone()).or_default() += 1;
        if matches!(toggle, Toggle::Played) {
            self.watched_pending += 1;
            self.watched_revision = self.watched_revision.wrapping_add(1);
        }
        let version = self.versions.entry(key.clone()).or_default();
        *version = version.wrapping_add(1);
        let refresh_version = self.refresh_versions.entry(key).or_default();
        *refresh_version = refresh_version.wrapping_add(1);
        *version
    }

    fn settle(&mut self, id: &str, toggle: Toggle) {
        let key = (id.to_string(), matches!(toggle, Toggle::Favorite));
        if let Some(pending) = self.pending.get_mut(&key) {
            *pending = pending.saturating_sub(1);
        }
        let refresh_version = self.refresh_versions.entry(key).or_default();
        *refresh_version = refresh_version.wrapping_add(1);
        if matches!(toggle, Toggle::Played) {
            self.watched_pending = self.watched_pending.saturating_sub(1);
            self.watched_revision = self.watched_revision.wrapping_add(1);
        }
    }

    fn has_pending(&self, id: &str, toggle: Toggle) -> bool {
        self.pending
            .get(&(id.to_string(), matches!(toggle, Toggle::Favorite)))
            .copied()
            .unwrap_or_default()
            > 0
    }

    pub fn snapshot(&self) -> HashMap<(String, bool), u64> {
        self.refresh_versions.clone()
    }

    pub fn preserve_refresh(
        &self,
        id: &str,
        snapshot: &HashMap<(String, bool), u64>,
        current: Option<&UserData>,
        incoming: &mut UserData,
    ) {
        for toggle in [Toggle::Played, Toggle::Favorite] {
            let key = (id.to_string(), matches!(toggle, Toggle::Favorite));
            let version = self.refresh_versions.get(&key).copied().unwrap_or_default();
            let started_with = snapshot.get(&key).copied().unwrap_or_default();
            if version != started_with || self.has_pending(id, toggle) {
                let pending = self.has_pending(id, toggle);
                if let Some(source) = current
                    .or_else(|| pending.then(|| self.optimistic.get(&key)).flatten())
                    .or_else(|| self.confirmed.get(&key))
                {
                    copy_toggle(incoming, source, toggle);
                }
            }
        }
    }

    pub fn watched_revision(&self) -> u64 {
        self.watched_revision
    }

    pub fn has_watched_pending(&self) -> bool {
        self.watched_pending > 0
    }

    fn confirmed(&self, id: &str, toggle: Toggle) -> Option<UserData> {
        self.confirmed
            .get(&(id.to_string(), matches!(toggle, Toggle::Favorite)))
            .cloned()
    }

    fn optimistic(&mut self, id: &str, toggle: Toggle, data: &UserData) {
        self.optimistic.insert(
            (id.to_string(), matches!(toggle, Toggle::Favorite)),
            data.clone(),
        );
    }

    fn confirm(&mut self, id: &str, toggle: Toggle, data: &UserData) {
        self.confirmed.insert(
            (id.to_string(), matches!(toggle, Toggle::Favorite)),
            data.clone(),
        );
    }

    fn is_current(&self, id: &str, toggle: Toggle, version: u64) -> bool {
        self.versions
            .get(&(id.to_string(), matches!(toggle, Toggle::Favorite)))
            == Some(&version)
    }

}

pub trait UserDataView: Sized + 'static {
    fn user_data(&mut self, id: &str) -> Option<&mut UserData>;
    fn mutations(&mut self) -> &mut UserDataMutations;
    fn set_error(&mut self, id: &str, toggle: Toggle, error: SharedString);
    /// Server accepted the change; e.g. Series detail reloads its Episodes.
    fn saved(&mut self, _id: &str, _toggle: Toggle, _cx: &mut Context<Self>) {}
    /// Server rejected the change; reconcile related views where needed.
    fn failed(&mut self, _id: &str, _toggle: Toggle, _cx: &mut Context<Self>) {}
}

fn copy_toggle(data: &mut UserData, source: &UserData, toggle: Toggle) {
    match toggle {
        Toggle::Played => {
            data.played = source.played;
            data.playback_position_ticks = source.playback_position_ticks;
            data.played_percentage = source.played_percentage;
        }
        Toggle::Favorite => data.is_favorite = source.is_favorite,
    }
}

/// Flips `toggle` optimistically, then serializes the per-item server mutation. A
/// failed request reconciles only its own field so independent toggles remain intact.
pub fn toggle<V: UserDataView>(
    view: &mut V,
    api: &Api,
    id: &str,
    toggle: Toggle,
    cx: &mut Context<V>,
) {
    let Some(before) = view.user_data(id).map(|data| Clone::clone(&*data)) else {
        return;
    };
    let version = view.mutations().begin(id, toggle, &before);
    let (previous, finished) = view.mutations().enqueue(id);
    let Some(data) = view.user_data(id) else {
        return;
    };
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
    let after = data.clone();
    view.mutations().optimistic(id, toggle, &after);
    cx.notify();
    let (api, id) = (api.clone(), id.to_string());
    cx.spawn(async move |this, cx| {
        if let Some(previous) = previous {
            let _ = previous.await;
        }
        let result = match toggle {
            Toggle::Played => api.set_played(&id, value).await,
            Toggle::Favorite => api.set_favorite(&id, value).await,
        };
        let recovered = if result.is_err() {
            api.item(&id).await.ok().map(|item| item.user_data)
        } else {
            None
        };
        this.update(cx, |view, cx| {
            view.mutations().settle(&id, toggle);
            match result {
                Ok(()) => {
                    view.mutations().confirm(&id, toggle, &after);
                    view.saved(&id, toggle, cx);
                }
                Err(err) => {
                    if let Some(remote) = &recovered {
                        view.mutations().confirm(&id, toggle, remote);
                    }
                    let confirmed = view.mutations().confirmed(&id, toggle);
                    if view.mutations().is_current(&id, toggle, version)
                        && let Some(data) = view.user_data(&id)
                    {
                        copy_toggle(
                            data,
                            recovered.as_ref().or(confirmed.as_ref()).unwrap_or(&before),
                            toggle,
                        );
                    }
                    let what = match toggle {
                        Toggle::Played => "watched state",
                        Toggle::Favorite => "favorite",
                    };
                    view.set_error(&id, toggle, format!("Could not update {what}: {err}").into());
                    view.failed(&id, toggle, cx);
                }
            }
            cx.notify();
        })
        .ok();
        drop(finished);
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::{UserDataMutations, copy_toggle};
    use crate::components::toggle_button::Toggle;
    use crate::jellyfin::UserData;

    #[test]
    fn failed_toggle_reconciliation_changes_only_its_own_state() {
        let mut data = UserData {
            played: true,
            playback_position_ticks: 500,
            played_percentage: Some(50.),
            is_favorite: true,
            ..UserData::default()
        };
        copy_toggle(
            &mut data,
            &UserData {
                is_favorite: false,
                ..UserData::default()
            },
            Toggle::Favorite,
        );
        assert!(data.played);
        assert_eq!(data.playback_position_ticks, 500);
        assert_eq!(data.played_percentage, Some(50.));
        assert!(!data.is_favorite);

        copy_toggle(
            &mut data,
            &UserData {
                played: false,
                playback_position_ticks: 120,
                played_percentage: Some(12.),
                ..UserData::default()
            },
            Toggle::Played,
        );
        assert!(!data.played);
        assert_eq!(data.playback_position_ticks, 120);
        assert_eq!(data.played_percentage, Some(12.));
        assert!(!data.is_favorite);
    }

    #[test]
    fn pending_mutations_protect_refreshes_and_only_watched_changes_invalidate_next_up() {
        let mut mutations = UserDataMutations::default();
        let user_data = UserData::default();
        let watched_revision = mutations.watched_revision();
        mutations.begin("item", Toggle::Favorite, &user_data);
        assert!(mutations.has_pending("item", Toggle::Favorite));
        assert_eq!(mutations.watched_revision(), watched_revision);
        mutations.settle("item", Toggle::Favorite);
        assert!(!mutations.has_pending("item", Toggle::Favorite));

        mutations.begin("item", Toggle::Played, &user_data);
        assert!(mutations.has_watched_pending());
        assert_ne!(mutations.watched_revision(), watched_revision);
        mutations.settle("item", Toggle::Played);
        assert!(!mutations.has_watched_pending());
    }

    #[test]
    fn refresh_merge_preserves_only_fields_mutated_since_request_started() {
        let mut mutations = UserDataMutations::default();
        let snapshot = mutations.snapshot();
        let before = UserData::default();
        mutations.begin("episode", Toggle::Played, &before);
        let played = UserData {
            played: true,
            ..UserData::default()
        };
        mutations.optimistic("episode", Toggle::Played, &played);
        let mut pending_refresh = UserData::default();
        mutations.preserve_refresh("episode", &snapshot, None, &mut pending_refresh);
        assert!(pending_refresh.played, "pending intent survives a season switch");
        mutations.confirm("episode", Toggle::Played, &played);
        mutations.settle("episode", Toggle::Played);

        let mut incoming = UserData {
            played: false,
            is_favorite: true,
            ..UserData::default()
        };
        mutations.preserve_refresh("episode", &snapshot, None, &mut incoming);
        assert!(incoming.played);
        assert!(incoming.is_favorite, "unmutated favorite remains fresh");

        let mut unrelated = UserData {
            played: false,
            is_favorite: true,
            ..UserData::default()
        };
        mutations.preserve_refresh("other", &snapshot, None, &mut unrelated);
        assert!(!unrelated.played);
        assert!(unrelated.is_favorite);
    }

    #[test]
    fn failed_latest_mutation_falls_back_to_last_confirmed_server_value() {
        let mut mutations = UserDataMutations::default();
        let initial = UserData::default();
        mutations.begin("item", Toggle::Played, &initial);
        let optimistic = UserData {
            played: true,
            ..UserData::default()
        };
        mutations.confirm("item", Toggle::Played, &optimistic);
        let latest = UserData {
            played: false,
            ..UserData::default()
        };
        let version = mutations.begin("item", Toggle::Played, &latest);
        mutations.settle("item", Toggle::Played);
        assert!(mutations.is_current("item", Toggle::Played, version));
        assert!(mutations.confirmed("item", Toggle::Played).unwrap().played);
    }

    #[test]
    fn queued_mutations_for_an_item_release_in_action_order() {
        let mut mutations = UserDataMutations::default();
        let (previous, first_finished) = mutations.enqueue("item");
        assert!(previous.is_none());
        let (previous, _second_finished) = mutations.enqueue("item");
        first_finished.send(()).unwrap();
        futures::executor::block_on(previous.unwrap()).unwrap();
    }
}
