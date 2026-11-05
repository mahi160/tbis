fn main() {
    // libmpv-sys only emits `rustc-link-lib=mpv`; this script adds the search path
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // no pkg-config here: scripts/setup-windows-deps.ps1 builds mpv.lib from pinned libmpv
        println!("cargo:rerun-if-env-changed=MPV_LIB_DIR");
        let dir =
            std::env::var("MPV_LIB_DIR").expect("MPV_LIB_DIR must name the folder holding mpv.lib");
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rerun-if-env-changed=TBIS_WINDOWS_ICON");
        println!("cargo:rerun-if-changed=Cargo.toml");
        let icon = std::env::var("TBIS_WINDOWS_ICON")
            .expect("Run scripts/setup-windows-deps.ps1 to generate TBIS_WINDOWS_ICON");
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
}
