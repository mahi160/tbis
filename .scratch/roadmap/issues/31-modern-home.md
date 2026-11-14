# 31 — Layout setting and Modern Home

**What to build:** A second Home layout the user can pick in Settings > Appearance > Layout: **Default** (today's Home) or **Modern**. Modern shows a hero, then Continue Watching and Next Up, then one row per Library with that Library's newest items. Both layouts share the same data loading; only the rendering differs.

**Model:** sonnet

**Blocked by:** 28, 29, 30, and owner decision 3 in `PLAN.md`. The layout below is the recommended one; confirm it before starting.

**Status:** blocked

**Read first:** `CONTEXT.md`, `AGENTS.md`, `src/home.rs` (all), `src/app.rs` (`Main`, `main_screen`, `fn open_settings`), `src/config.rs`, `src/settings.rs` (`fn appearance_page`), `src/components/hero.rs`, `src/components/row.rs`.

## Steps

1. **Words first.** `CONTEXT.md` says the app always merges every Library. Update the **Library** entry: the Default layout merges every Library; the Modern layout shows one row per Library. Add a **Hero** entry under Home: the featured item at the top of the Modern layout.
2. **Setting.** In `src/config.rs` add `enum HomeLayout { #[default] Default, Modern }` with `ALL` and `label`, a `Config` field with `#[serde(default)]`, and the entry in `fn defaults`. Wire it through Settings like `VideoQuality` in ticket 09 (`SettingsChanged::Layout`, entity, observer, dropdown in `fn appearance_page`, match arm in `open_settings`).
3. **Data.** Extend `HomeView`'s loaded data with `libraries: Vec<(Library, Vec<Item>)>`. Load it only when the layout is Modern: `api.libraries()`, then `api.latest_in` for each Library, all inside the existing background task. A failure to load Libraries shows the existing Home error with Retry.
4. **Hero item.** The first Continue Watching item; when there is none, the first item of the first Library row. Backdrop from `api.backdrop_url`, logo from `api.logo_url`. Play emits `Nav::Play`, More info opens the detail page the way a card click does.
5. **Rendering.** In `HomeView::render`, branch on the layout. Default calls the existing code unchanged. Modern builds: hero, Continue Watching, Next Up, then a `paged_row` per Library titled with the Library's name. Each row needs its own `ScrollHandle`: change `scroll: [ScrollHandle; 4]` to a `Vec<ScrollHandle>` sized to the row count, and keep paging working for both layouts.
6. **Switching.** When the setting changes, `open_settings`'s arm tells the Home view (`main.home.update(...)`) the new layout and marks it stale so the next visit loads the Library rows.
7. Remove the `#[allow(dead_code)]` markers added in tickets 28 and 30.
8. Keep the loading skeleton and the empty state working in Modern (`components/status.rs`).

## Done when

- [ ] Settings > Appearance > Layout switches between Default and Modern, saved to `config.json`
- [ ] Default looks exactly as before
- [ ] Modern shows the hero, Continue Watching, Next Up and one row per Movie or Series Library, each with working page arrows
- [ ] Hero Play starts playback; More info opens the detail page
- [ ] A server with one Library and a server with none both render without errors
- [ ] `CONTEXT.md` reflects the new words
- [ ] Old configs load as Default (test)

**Check:** the gate in `PLAN.md`, then screenshots of both layouts on each theme. Follow the build-freshness rule in `AGENTS.md`.
