fn main() {
    // ADR-0002: link user's brew libmpv, never bundle
    pkg_config::probe_library("mpv").expect("libmpv not found via pkg-config (brew install mpv)");
}
