use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // build links brew's libmpv; scripts/bundle.sh copies it into the .app (ADR-0004)
    pkg_config::probe_library("mpv").expect("libmpv not found via pkg-config (brew install mpv)");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        build_libtailscale();
    }
}

/// Compiles the vendored libtailscale (Go `tsnet`) into a static C archive and links it (ADR-0007).
fn build_libtailscale() {
    let src = Path::new("vendor/libtailscale");
    for file in [
        "tailscale.go",
        "tailscale.c",
        "tailscale.h",
        "go.mod",
        "go.sum",
    ] {
        println!("cargo:rerun-if-changed={}", src.join(file).display());
    }
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
    if !src.join("tailscale.go").exists() {
        panic!("vendor/libtailscale is missing: run `git submodule update --init`");
    }
    if Command::new("go").arg("version").output().is_err() {
        panic!(
            "Go toolchain not found: building libtailscale needs `go` on PATH (brew install go, ADR-0007)"
        );
    }
    let goarch = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        arch => panic!("libtailscale: unsupported target arch {arch}"),
    };
    let min_macos = std::env::var("MACOSX_DEPLOYMENT_TARGET")
        .expect("MACOSX_DEPLOYMENT_TARGET is set in .cargo/config.toml");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let status = Command::new("go")
        .current_dir(src)
        .args(["build", "-buildmode=c-archive", "-trimpath", "-o"])
        .arg(out.join("libtailscale.a"))
        .env("CGO_ENABLED", "1")
        .env("GOOS", "darwin")
        .env("GOARCH", goarch)
        .env("MACOSX_DEPLOYMENT_TARGET", &min_macos)
        .env("CGO_CFLAGS", format!("-mmacos-version-min={min_macos}"))
        .env("CGO_LDFLAGS", format!("-mmacos-version-min={min_macos}"))
        .status()
        .expect("failed to run `go build` for libtailscale");
    assert!(status.success(), "`go build` of libtailscale failed");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=tailscale");
    for framework in ["CoreFoundation", "Security"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
