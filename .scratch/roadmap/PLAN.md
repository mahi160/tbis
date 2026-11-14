# tbis roadmap

The plan to make tbis the best desktop Jellyfin player: it plays everything on macOS and Windows, feels instant, never hides an error, and stays small.

Each ticket in `issues/` is one self-contained change. Work one ticket per session, in the order below.

## How to work a ticket

1. Read `AGENTS.md` and `CONTEXT.md`, then the ticket, then every file under its **Read first**.
2. Make only the change the ticket describes. Anchors are function names, because line numbers drift; find them with `rg -n "fn name" src`.
3. Run the ticket's **Check**. Unless the ticket says otherwise, the check is the **gate**:

   ```sh
   cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
   ```

   The gate must end with `test result: ok` and no warnings.
4. Tick the ticket's boxes, set its **Status** to `done` (add `runtime unverified` when nobody watched it run), and stop. Leave committing to the owner.

House rules that apply to every ticket:

- UI is built from `src/components/`; colors, sizes and gaps come from `src/design.rs` (see `AGENTS.md`).
- Fix the code, never the tests. A failing gate means the change is wrong.
- Use only APIs you have read in this repo or in `~/.cargo/registry/src/*/gpui-pre-0.3.9/`. When a ticket names an mpv option, confirm it with `mpv --list-options | rg <name>` first.
- Windows-only code (`src/platform/windows/`) does not compile on a Mac. Its check is the **Windows** workflow in GitHub Actions (`.github/workflows/windows.yml`, manual dispatch). Say `Windows CI not run` in the Status when you could not run it.
- When a ticket's instructions do not match the code you find, stop and write what differs under the ticket's Status instead of guessing.

## Model guide

| Label | Meaning |
| --- | --- |
| `haiku` | Steps are exact; follow them. |
| `sonnet` | Needs reading and some judgement inside one or two files. |
| `opus + human` | Design or hardware work; a person must run it on real machines. |

## Order

### A. Windows plays video (do first)

| # | Ticket | Model | Status |
| --- | --- | --- | --- |
| 01 | Windows install docs name the right download | haiku | done |
| 02 | Log every Player error | haiku | done |
| 03 | mpv logs at `info` | haiku | done |
| 04 | Error panel shows the cause and offers Retry | haiku | done |
| 05 | Log the Windows GPU at Player start | sonnet | done |
| 06 | Run tbis on a real Windows PC and record the log | human | needs-human |
| 07 | Spike: mpv draws into a child window on Windows | opus + human | blocked by 06 |
| 08 | Windows libmpv from a prebuilt package | opus | blocked by owner decision |

### B. mpv quality

| # | Ticket | Model | Status |
| --- | --- | --- | --- |
| 09 | Video quality setting (Fast / Balanced / High) | haiku | done |
| 10 | Exclusive audio only with passthrough | haiku | done |

### C. Player

| # | Ticket | Model | Status |
| --- | --- | --- | --- |
| 11 | On-screen feedback for seek, speed and mute | haiku | done |
| 12 | Click to pause, double-click fullscreen, wheel volume | haiku | done |
| 13 | Buffered range on the timeline | sonnet | done |
| 14 | Thumbnail preview while hovering the timeline | sonnet | ready |
| 15 | Auto-skip intros and credits | haiku | ready |

### D. Settings

| # | Ticket | Model | Status |
| --- | --- | --- | --- |
| 16 | Reset settings | haiku | ready |
| 17 | Better defaults | haiku | blocked by owner decision |

### E. Performance

| # | Ticket | Model | Status |
| --- | --- | --- | --- |
| 18 | Tabs reload only when stale | haiku | ready |
| 19 | Home decodes off the UI thread | haiku | ready |
| 20 | Bounded image memory on Home and Search | haiku | ready |
| 21 | Sharper wide art on Retina | haiku | ready |
| 22 | Debounce mpv.conf saves | haiku | ready |
| 23 | Load config once at launch | haiku | ready |

### F. Safety and tidiness

| # | Ticket | Model | Status |
| --- | --- | --- | --- |
| 24 | Log dropped mpv errors in the Player | haiku | ready |
| 25 | Warn about unencrypted remote servers | haiku | ready |
| 26 | Split `jellyfin.rs` into modules | sonnet | ready |
| 27 | Signed releases | human | needs-human |

### G. Modern view (parked until the owner picks the layout)

| # | Ticket | Model | Status |
| --- | --- | --- | --- |
| 28 | Jellyfin: Libraries and per-Library queries | sonnet | ready |
| 29 | Reusable paged row component | haiku | ready |
| 30 | Hero component | sonnet | blocked by 28 |
| 31 | Layout setting and Modern Home | sonnet | blocked by 28, 29, 30 and owner decision |

## Owner decisions still open

1. **Windows libmpv (ticket 08):** prebuilt package (full codecs, short CI, source by link) or keep the source build (strict source mirror, 1.4 GB archive). Recommended: prebuilt.
2. **Defaults (ticket 17):** tick the rows you want in the ticket's table.
3. **Modern layout (ticket 31):** one row per Library under a hero, or Movies and Series tabs each split by Library. Recommended: one row per Library.

## Already decided

- Embedded Tailscale is removed on every platform and will be planned again separately. A server on a tailnet works today when the Tailscale app is running: type its tailnet address as the server address.
- The access token stays in `config.json` (mode 0600 on Unix).
- The 1.4 GB release asset is `tbis-<version>-windows-sources.tar`, the GPL source archive. The app is `tbis-<version>-windows-x86_64.zip` (about 27 MB).

## Facts worth knowing before you start

- macOS and Wayland draw video through mpv's render API (`src/mpv/render.rs`, `src/platform/*/video*`). Windows does the same through WGL and DX interop, and that path has never been confirmed on real hardware (`docs/windows.md`).
- gpui's Windows renderer creates its composition target with `topmost = true` (`gpui-pre-windows-0.3.8/src/directx_renderer.rs`, `CreateTargetForHwnd(hwnd, true)`), which is what makes ticket 07 plausible.
- A time update re-rendering only a child entity does not help: gpui marks every ancestor dirty (`.scratch/perf-pip-changelog/issues/01-timeline-only-redraw.md`).
- Local mpv is 0.41 (`mpv --profile=help` lists `fast` and `high-quality`); Windows pins 0.42 (`scripts/windows-deps.json`).
