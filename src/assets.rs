use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

/// Reicon icons used by the Player (assets/icons/LICENSE-REICON).
const EXTRA: &[(&str, &[u8])] = &[
    (
        "icons/caret-left.svg",
        include_bytes!("../assets/icons/caret-left.svg"),
    ),
    ("icons/cc.svg", include_bytes!("../assets/icons/cc.svg")),
    (
        "icons/cc-filled.svg",
        include_bytes!("../assets/icons/cc-filled.svg"),
    ),
    (
        "icons/forward-step.svg",
        include_bytes!("../assets/icons/forward-step.svg"),
    ),
    (
        "icons/headphones.svg",
        include_bytes!("../assets/icons/headphones.svg"),
    ),
    (
        "icons/maximize.svg",
        include_bytes!("../assets/icons/maximize.svg"),
    ),
    (
        "icons/minimize.svg",
        include_bytes!("../assets/icons/minimize.svg"),
    ),
    ("icons/mute.svg", include_bytes!("../assets/icons/mute.svg")),
    (
        "icons/pause.svg",
        include_bytes!("../assets/icons/pause.svg"),
    ),
    ("icons/pip.svg", include_bytes!("../assets/icons/pip.svg")),
    ("icons/play.svg", include_bytes!("../assets/icons/play.svg")),
    (
        "icons/volume.svg",
        include_bytes!("../assets/icons/volume.svg"),
    ),
];

/// gpui-kit's default icons plus [`EXTRA`].
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match EXTRA.iter().find(|(p, _)| *p == path) {
            Some((_, bytes)) => Ok(Some(Cow::Borrowed(bytes))),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(
            EXTRA
                .iter()
                .filter(|(p, _)| p.starts_with(path))
                .map(|(p, _)| (*p).into()),
        );
        Ok(paths)
    }
}
