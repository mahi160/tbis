# 31 — Layout setting and Modern Home

**What to build:** A second Home layout the user can pick in Settings > Appearance > Layout: **Default** (today's Home) or **Modern**. Modern shows a hero, then Continue Watching and Next Up, then one row per Library with that Library's newest items. Both layouts share the same data loading; only the rendering differs.

**Model:** sonnet

**Blocked by:** 28, 29, 30, and owner decision 3 in `PLAN.md`. The layout below is the recommended one; confirm it before starting.

**Status:** done, partly runtime unverified. Differences from the steps:
- List items carry no logo tag or overview, so the Hero fetches its item in full with `api.item`. For an Episode it shows its Series' art and overview, with `S01E02 · Name` as meta; Play still plays the Episode. A failed lookup falls back to the list item.
- When nothing is in Continue Watching, the Hero shows the first item of the first non-empty Library.
- The loading task is shared, but each layout fetches only the rows it shows. The empty state counts only those rows.
- Library rows include played items, because `latest_in` doesn't filter them (ticket 28). So the fallback Hero can be a played item.
- Row scroll handles are keyed by row id, so a row keeps its scroll position when the Library list changes.
- The loading skeleton has no hero slot, so the page shifts when the hero appears.

**Owner follow-up:** the single-item Hero became a carousel.
- **Content:** the five newest unstarted Movies, each fetched in full with `api.item`. It no longer uses Continue Watching or Library items.
- **Motion:** it auto-advances every 8 s, pauses while the pointer is on it, and has dots below. A change slides the whole strip one step, the short way round, over 600 ms: the arriving film grows from peek size, brightens and fades its text in, and the leaving one shrinks and dims. With reduced motion on, the slide changes in place.
- **Size:** the hero is cinema-wide (2.4:1) and centred once the window is wider than its cap, with a minimum height for narrow windows.
- **Type and buttons:** two type sizes are added for the hero only (`text::lead`, `text::display`), plus an `xl` button size and `glass_button` for More info.
- **Look:** it follows the research (Apple Top Shelf, Netflix, Stremio): one meta line with age rating and format pills, a two-line overview, solid Play beside a translucent More info, and soft left and bottom scrims. The meta line has no middle-dot separators, and the rating uses the shared star.
- **Corners:** every painted layer (art and scrims) is rounded itself, because gpui's `overflow_hidden` clips to a rectangle, not to rounded corners. The real cause of the square bottom corners: gpui's `img` takes the picture's own aspect ratio (16:9), so in the 2.4:1 card the art came out taller than the card and was cut off square. `art::image` is now absolute-fill, so every image takes its box's height; `art::thumb` became `relative` for it.
- **Peeks:** the current slide takes 84% of the width (capped). The previous and next films' art shows at 82% size, dimmed and cut off at each edge, with a tooltip, and clicking one shows it. Slides are placed in pixels from the window width (as `poster_columns` does), so position, size and dimming can move together. Two more films sit off screen to slide in.
- **Countdown:** the active dot is a pill that fills over the 8 s wait. It shows full while the pointer pauses the carousel, and leaving restarts the full wait. Each dot's tooltip shows its film's title.
- **Checked on screen:** dark and light themes, windows at 900, 1200 and 2000 pt wide, auto-advance, clicking a dot, and More info opening the detail page.
- `Hero` now adds its own page gutter, so it lines up with the rows.

Screenshots taken: Default and Modern on all four themes, plus Modern scrolled down to its Library rows on Raptor Night. Default matches the old Home. Not run: switching the setting live, clicking Hero Play or More info, the page arrows in Modern, and servers with one Library or none.

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
- [x] Default looks exactly as before
- [ ] Modern shows the hero, Continue Watching, Next Up and one row per Movie or Series Library, each with working page arrows
- [ ] Hero Play starts playback; More info opens the detail page
- [ ] A server with one Library and a server with none both render without errors
- [x] `CONTEXT.md` reflects the new words
- [x] Old configs load as Default (test)

**Check:** the gate in `PLAN.md`, then screenshots of both layouts on each theme. Follow the build-freshness rule in `AGENTS.md`.
