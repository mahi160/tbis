//! Update check, offer and install for `AppView`.

use super::*;
use crate::components::dialog::{release_notes, vega_dialog};
use crate::update;
use gpui_kit::component::dialog::DialogButtonProps;

impl AppView {
    /// Looks for a newer release; `manual` (menu) also reports "up to date" and errors.
    pub fn check_for_updates(&mut self, manual: bool, window: &mut Window, cx: &mut Context<Self>) {
        if update::installed().is_none() {
            if manual {
                let note =
                    Notification::info("Automatic updates are unavailable for this installation.");
                window.push_notification(note, cx);
            }
            return;
        }
        let http = cx.http_client();
        self._update = cx.spawn_in(window, async move |this, cx| {
            let result = update::check(http).await;
            this.update_in(cx, |this, window, cx| match result {
                Ok(Some(update)) => this.offer_update(update, window, cx),
                Ok(None) if manual => {
                    let note = format!("tbis {} is the latest version.", update::CURRENT);
                    window.push_notification(Notification::success(note), cx);
                }
                Ok(None) => {}
                Err(err) if manual => {
                    let note = Notification::error(format!("Update check failed: {err}"));
                    window.push_notification(note, cx);
                }
                // automatic check: offline or rate-limited is no reason to interrupt
                Err(err) => crate::log::write(&format!("update check failed: {err}")),
            })
            .ok();
        });
    }

    fn offer_update(
        &mut self,
        update: update::Update,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let this = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, cx| {
            let (this, update) = (this.clone(), update.clone());
            vega_dialog(alert)
                .title(format!("tbis {} is available", update.version))
                .description(format!(
                    "You have {}. tbis restarts to finish updating.",
                    update::CURRENT
                ))
                .child(release_notes(
                    update.notes.clone().into(),
                    update.page.clone().into(),
                    cx,
                ))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Install and Restart")
                        .cancel_text("Later"),
                )
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    let update = update.clone();
                    this.update(cx, |this, cx| this.install_update(update, window, cx))
                        .ok();
                    true
                })
        });
    }

    fn install_update(
        &mut self,
        update: update::Update,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let note = Notification::info(format!("Downloading tbis {}\u{2026}", update.version));
        window.push_notification(note, cx);
        let http = cx.http_client();
        self._update = cx.spawn_in(window, async move |this, cx| {
            let installed = cx
                .background_spawn(async move { update::install(http, &update).await })
                .await;
            let result = installed.map(|()| update::relaunch());
            this.update_in(cx, |_, window, cx| match result {
                Ok(Ok(())) => cx.quit(),
                Ok(Err(err)) => {
                    crate::log::write(&format!("relaunch after update failed: {err}"));
                    let note = Notification::info(
                        "The update is installed. Quit and reopen tbis to finish.",
                    );
                    window.push_notification(note, cx);
                }
                Err(err) => {
                    let note = Notification::error(format!("Update failed: {err}"));
                    window.push_notification(note, cx);
                }
            })
            .ok();
        });
    }
}
