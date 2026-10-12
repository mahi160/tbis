/// Hides the pointer until the mouse moves again.
pub fn hide() {
    objc2_app_kit::NSCursor::setHiddenUntilMouseMoves(true);
}
