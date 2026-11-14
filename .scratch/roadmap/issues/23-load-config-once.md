# 23 — Load config once at launch

**What to build:** `config.json` is read and parsed three times during launch: twice in `src/main.rs` (theme, window bounds) and once in `AppView::new`. Read it once and pass it along.

**Model:** haiku

**Blocked by:** None.

**Status:** ready-for-agent

**Read first:** `src/main.rs` (the two `config::load()` calls and the window-opening code between them), `src/app.rs` (`AppView::new`), `src/config.rs` (`pub fn load`).

## Steps

1. In `src/main.rs`, call `let config = config::load();` once, before the theme is initialised.
2. Use `config.theme.as_deref()` for `theme::init` and `config.window` for the saved window bounds.
3. Change `AppView::new` to take the config: `pub fn new(config: Config, window: &mut Window, cx: &mut Context<Self>) -> Self`, and delete its own `config::load()` line. Move `config` into the closure that creates the view. When the closure can run more than once, wrap the config in `Option` and `take()` it; read the surrounding code to decide.
4. Run `rg -n "config::load\(\)" src`. Exactly one call must remain outside tests.

## Done when

- [ ] One `config::load()` call at launch
- [ ] Theme, window size and session restore behave as before
- [ ] The "Could not load saved configuration" notification still appears for a broken file

**Check:** the gate in `PLAN.md`. Runtime: launch, confirm the saved theme and window size are restored. Then put invalid JSON in the dev `config.json`, launch, confirm the notification, and restore the file.
