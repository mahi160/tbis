//! Shared UI pieces every screen builds from. Each wraps a gpui-kit component with
//! the app's defaults, or fills a gap gpui-kit doesn't cover (poster art, backdrops).
//! Pages compose these; they don't hand-build UI inline.

pub mod ambient;
pub mod art;
pub mod avatar;
pub mod button;
pub mod card;
pub mod chart;
pub mod detail;
pub mod dialog;
pub mod episode;
pub mod frost;
pub mod input;
pub mod layout;
pub mod list;
pub mod logo;
pub mod menu;
pub mod panel;
pub mod section;
pub mod settings;
pub mod shortcuts;
pub mod status;
pub mod tabs;
pub mod tag;
pub mod title_bar;
pub mod toggle_button;
pub mod typography;
pub mod video;

pub type OnClick = Box<dyn Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App)>;
