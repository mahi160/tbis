use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // libmpv-sys only emits `rustc-link-lib=mpv`; this script adds the search path
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // no pkg-config here: scripts/setup-windows-deps.ps1 builds mpv.lib from pinned libmpv
        println!("cargo:rerun-if-env-changed=MPV_LIB_DIR");
        let dir =
            std::env::var("MPV_LIB_DIR").expect("MPV_LIB_DIR must name the folder holding mpv.lib");
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rerun-if-changed=Cargo.toml");
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("Cargo manifest directory");
        let icon = format!("{manifest_dir}/assets/icon.ico");
        println!("cargo:rerun-if-changed={icon}");
        let version = std::env::var("CARGO_PKG_VERSION").expect("Cargo package version");
        let numeric = format!("{},0", version.replace('.', ","));
        let resource = format!(
            r#"#include <windows.h>
1 ICON "{}"
1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0
FILEOS VOS_NT_WINDOWS32
FILETYPE VFT_APP
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "CompanyName", "tbis"
            VALUE "FileDescription", "tbis"
            VALUE "FileVersion", "{version}"
            VALUE "InternalName", "tbis"
            VALUE "OriginalFilename", "tbis.exe"
            VALUE "ProductName", "tbis"
            VALUE "ProductVersion", "{version}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#,
            icon.replace('\\', "/")
        );
        let out =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
        let rc = out.join("tbis.rc");
        std::fs::write(&rc, resource).expect("Write Windows resources");
        embed_resource::compile(&rc, embed_resource::NONE)
            .manifest_required()
            .expect("Embed Windows icon and version");
        return;
    }
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
