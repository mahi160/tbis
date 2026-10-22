//! Keyboard shortcuts help (`?`): lists each action with the keys the keymap
//! actually binds it to, so the list follows any rebinding. New shortcuts need a
//! row in `GROUPS` to be listed. `{short}`/`{long}` in a label become the
//! configured seek steps.

use gpui_kit::component::WindowExt as _;
use gpui_kit::component::kbd::Kbd;
use gpui_kit::*;

use crate::components::shortcuts::shortcut_list;

use crate::config::SeekSteps;
use crate::player::*;

actions!(tbis, [ShowShortcuts]);

type Row = (fn() -> Box<dyn Action>, &'static str);

const GROUPS: &[(&str, &[Row])] = &[
    (
        "Playback",
        &[
            (|| Box::new(TogglePause), "Play / pause"),
            (|| Box::new(SeekBack), "Back {short} s"),
            (|| Box::new(SeekForward), "Forward {short} s"),
            (|| Box::new(SeekBackLong), "Back {long} s"),
            (|| Box::new(SeekForwardLong), "Forward {long} s"),
            (|| Box::new(ChapterPrev), "Previous chapter"),
            (|| Box::new(ChapterNext), "Next chapter"),
            (|| Box::new(SkipSegment), "Skip intro / credits"),
            (|| Box::new(PreviousEpisode), "Previous Episode"),
            (|| Box::new(NextEpisode), "Next Episode"),
            (
                || Box::new(PlayNext),
                "Play next Episode now (up-next card)",
            ),
            (|| Box::new(SpeedDown), "Slower"),
            (|| Box::new(SpeedUp), "Faster"),
            (|| Box::new(VolumeUp), "Volume up"),
            (|| Box::new(VolumeDown), "Volume down"),
            (|| Box::new(ToggleMute), "Mute"),
            (|| Box::new(ToggleFullscreen), "Fullscreen"),
            (|| Box::new(TogglePip), "Picture-in-Picture"),
            (|| Box::new(TogglePlaybackInfo), "Playback info"),
            (|| Box::new(Screenshot), "Screenshot"),
            (|| Box::new(CycleShaders), "Cycle upscaling"),
            (|| Box::new(Escape), "Exit fullscreen / cancel"),
        ],
    ),
    (
        "Tracks",
        &[
            (|| Box::new(CycleAudio), "Next audio track"),
            (|| Box::new(CycleSubtitle), "Next subtitle track"),
            (|| Box::new(SubDelayEarlier), "Subtitles earlier"),
            (|| Box::new(SubDelayLater), "Subtitles later"),
            (|| Box::new(AudioDelayEarlier), "Audio earlier"),
            (|| Box::new(AudioDelayLater), "Audio later"),
        ],
    ),
    (
        "App",
        &[
            (|| Box::new(crate::app::FocusSearch), "Search"),
            (|| Box::new(crate::app::GoBack), "Back"),
            (|| Box::new(crate::app::ShowHome), "Home"),
            (|| Box::new(crate::app::ShowMovies), "Movies"),
            (|| Box::new(crate::app::ShowSeries), "Series"),
            (|| Box::new(ShowShortcuts), "Keyboard shortcuts"),
            (|| Box::new(crate::CloseWindow), "Close window"),
            (|| Box::new(crate::Hide), "Hide"),
            (|| Box::new(crate::Quit), "Quit"),
        ],
    ),
];

pub fn bind_keys(cx: &mut App) {
    // not while typing: `?` belongs to the search field there. Both spellings:
    // unverified which one macOS reports; last one is what the help shows
    cx.bind_keys([
        KeyBinding::new("shift-/", ShowShortcuts, Some("!Input")),
        KeyBinding::new("?", ShowShortcuts, Some("!Input")),
    ]);
}

pub fn open(window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        window.close_dialog(cx);
        return;
    }
    window.open_dialog(cx, |dialog, _, cx| {
        crate::components::dialog::vega_dialog(dialog)
            .title("Keyboard shortcuts")
            .w(crate::design::size::dialog_w())
            .child(content(cx))
    });
}

fn content(cx: &App) -> impl IntoElement {
    let steps = cx.global::<SeekSteps>();
    let label = |text: &str| {
        text.replace("{short}", &steps.short.to_string())
            .replace("{long}", &steps.long.to_string())
    };
    let keymap = cx.key_bindings();
    let keymap = keymap.borrow();
    let groups = GROUPS
        .iter()
        .map(|(group, rows)| {
            let bound = rows
                .iter()
                .filter_map(|(action, label_text)| {
                    // last binding wins, like the keymap itself
                    let action = action();
                    let binding = keymap.bindings_for_action(action.as_ref()).last()?;
                    let keys: Vec<Kbd> = binding
                        .keystrokes()
                        .iter()
                        .map(|k| Kbd::new(k.inner().clone()))
                        .collect();
                    Some((label(label_text), keys))
                })
                .collect();
            (*group, bound)
        })
        .collect();
    shortcut_list(groups, cx)
}
