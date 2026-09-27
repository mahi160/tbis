//! Bundled upscaling shaders: Anime4K v4.0.1 (MIT, assets/shaders/anime4k/LICENSE),
//! in its official "Mode A" chains. mpv reads shader files from disk, so
//! [`extract_dir`] writes them under Application Support, like `fonts.rs` does.

use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::embed::embedded_by_name;

const FILES: &[(&str, &[u8])] = &embedded_by_name!(
    "shaders/anime4k",
    [
        "Anime4K_Clamp_Highlights.glsl",
        "Anime4K_Restore_CNN_M.glsl",
        "Anime4K_Restore_CNN_VL.glsl",
        "Anime4K_Upscale_CNN_x2_S.glsl",
        "Anime4K_Upscale_CNN_x2_M.glsl",
        "Anime4K_Upscale_CNN_x2_VL.glsl",
        "Anime4K_AutoDownscalePre_x2.glsl",
        "Anime4K_AutoDownscalePre_x4.glsl",
    ]
);

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum ShaderProfile {
    #[default]
    Off,
    /// Mode A with M/S networks: light enough for integrated GPUs.
    Anime4kFast,
    /// Mode A with VL networks: sharper, needs a strong GPU.
    Anime4kQuality,
}

impl ShaderProfile {
    pub const ALL: [Self; 3] = [Self::Off, Self::Anime4kFast, Self::Anime4kQuality];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Anime4kFast => "Anime4K (fast)",
            Self::Anime4kQuality => "Anime4K (quality)",
        }
    }

    /// Next in Off, fast, quality order, for the Player's cycle key.
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Anime4kFast,
            Self::Anime4kFast => Self::Anime4kQuality,
            Self::Anime4kQuality => Self::Off,
        }
    }

    fn chain(self) -> &'static [&'static str] {
        match self {
            Self::Off => &[],
            Self::Anime4kFast => &[
                "Anime4K_Clamp_Highlights.glsl",
                "Anime4K_Restore_CNN_M.glsl",
                "Anime4K_Upscale_CNN_x2_M.glsl",
                "Anime4K_AutoDownscalePre_x2.glsl",
                "Anime4K_AutoDownscalePre_x4.glsl",
                "Anime4K_Upscale_CNN_x2_S.glsl",
            ],
            Self::Anime4kQuality => &[
                "Anime4K_Clamp_Highlights.glsl",
                "Anime4K_Restore_CNN_VL.glsl",
                "Anime4K_Upscale_CNN_x2_VL.glsl",
                "Anime4K_AutoDownscalePre_x2.glsl",
                "Anime4K_AutoDownscalePre_x4.glsl",
                "Anime4K_Upscale_CNN_x2_M.glsl",
            ],
        }
    }

    /// `glsl-shaders` value: `:`-separated paths, `""` for Off. Errors when the
    /// files could not be written, so callers fall back to no shaders.
    pub fn mpv_value(self) -> Result<String, String> {
        if self == Self::Off {
            return Ok(String::new());
        }
        let dir = extract_dir().ok_or("could not write shader files")?;
        Ok(self
            .chain()
            .iter()
            .map(|name| dir.join(name).to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(":"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_chained_shader_is_bundled() {
        for profile in ShaderProfile::ALL {
            for name in profile.chain() {
                assert!(FILES.iter().any(|(file, _)| file == name), "{name}");
            }
        }
    }
}

/// Written once per process; `None` if any write failed.
fn extract_dir() -> Option<PathBuf> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = crate::support_dir::app_support_dir().join("shaders");
        std::fs::create_dir_all(&dir).ok()?;
        for (name, bytes) in FILES {
            std::fs::write(dir.join(name), bytes).ok()?;
        }
        Some(dir)
    })
    .clone()
}
