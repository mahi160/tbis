fn main() {
    // build links brew's libmpv; scripts/bundle.sh copies it into the .app (ADR-0004)
    pkg_config::probe_library("mpv").expect("libmpv not found via pkg-config (brew install mpv)");
}
