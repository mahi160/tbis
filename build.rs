fn main() {
    // libmpv-sys only emits `rustc-link-lib=mpv`; this script adds the search path
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // no pkg-config here: CI builds mpv.lib from a pinned libmpv dll (.github/workflows/windows.yml)
        println!("cargo:rerun-if-env-changed=MPV_LIB_DIR");
        let dir = std::env::var("MPV_LIB_DIR")
            .expect("MPV_LIB_DIR must name the folder holding mpv.lib");
        println!("cargo:rustc-link-search=native={dir}");
        return;
    }
    // build links brew's libmpv; scripts/bundle.sh copies it into the .app (ADR-0004)
    pkg_config::probe_library("mpv").expect("libmpv not found via pkg-config (brew install mpv)");
}
