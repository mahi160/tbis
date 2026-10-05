//! gpui-kit dialogs overridden to shadcn vega's dialog: `rounded-xl`, `sm:max-w-md`.

use gpui_kit::*;

use crate::design::radius;

/// Applies the vega frame to any gpui-kit Dialog or AlertDialog.
pub fn vega_dialog<D: Styled>(dialog: D) -> D {
    dialog.rounded(radius::xl())
}
