use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

use crate::embed::embedded_by_path;

/// Reicon icons used by the Player (assets/icons/LICENSE-REICON).
const EXTRA: &[(&str, &[u8])] = &embedded_by_path!(
    "icons",
    [
        "caret-left.svg",
        "cc.svg",
        "cc-filled.svg",
        "forward-step.svg",
        "headphones.svg",
        "maximize.svg",
        "minimize.svg",
        "mute.svg",
        "pause.svg",
        "pip.svg",
        "play.svg",
        "volume.svg",
    ]
);

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
