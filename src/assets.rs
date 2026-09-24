use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

/// Lucide icons missing from gpui-kit's default bundle (assets/icons/LICENSE-LUCIDE).
const EXTRA: &[(&str, &[u8])] = &[
    (
        "icons/audio-lines.svg",
        include_bytes!("../assets/icons/audio-lines.svg"),
    ),
    (
        "icons/captions.svg",
        include_bytes!("../assets/icons/captions.svg"),
    ),
    (
        "icons/picture-in-picture-2.svg",
        include_bytes!("../assets/icons/picture-in-picture-2.svg"),
    ),
    (
        "icons/volume-2.svg",
        include_bytes!("../assets/icons/volume-2.svg"),
    ),
    (
        "icons/volume-x.svg",
        include_bytes!("../assets/icons/volume-x.svg"),
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
