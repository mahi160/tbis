#!/bin/sh
# Builds target/tbis.app, self-contained (ADR-0004): Homebrew's libmpv, the mpv
# binary PiP spawns (ADR-0003), and every Homebrew dylib they load are copied into
# the bundle and relinked, with their license files. Needs `brew install mpv` on
# the build machine only.
set -eu
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
app=target/tbis.app
frameworks="$app/Contents/Frameworks"
licenses="$app/Contents/Resources/licenses"
brew_prefix=$(brew --prefix)

cargo build --release

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$frameworks" "$licenses"
cp target/release/tbis "$app/Contents/MacOS/tbis"
cp "$brew_prefix/opt/mpv/bin/mpv" "$app/Contents/MacOS/mpv"
cp LICENSE "$app/Contents/Resources/LICENSE"

# Copies the formula's license files (Cellar/<formula>/<version>/) for `$1`, a
# resolved Homebrew path, once per formula.
copy_licenses() {
    formula_dir=$(echo "$1" | sed -n 's|^\(.*/Cellar/[^/]*/[^/]*\)/.*|\1|p')
    [ -n "$formula_dir" ] || return 0
    dest="$licenses/$(basename "$(dirname "$formula_dir")")-$(basename "$formula_dir")"
    [ -d "$dest" ] && return 0
    mkdir -p "$dest"
    find "$formula_dir" -maxdepth 1 -type f \( -iname 'LICEN[SC]E*' -o -iname 'COPYING*' \
        -o -iname 'COPYRIGHT*' -o -iname 'NOTICE*' -o -iname 'LGPL*' -o -iname 'GPL*' \) \
        -exec cp {} "$dest/" \;
}

# Points every Homebrew dylib `$1` loads at a bundled copy (@rpath/<name>), copying
# and relinking each new one recursively. Subshell body: recursion keeps its own vars.
bundle_deps() (
    file=$1
    origin=$2 # directory the file came from, for @loader_path references
    otool -L "$file" | tail -n +2 | awk '{print $1}' | while read -r dep; do
        case $dep in
            "$brew_prefix"/*) src=$dep ;;
            @loader_path/*) src="$origin/${dep#@loader_path/}" ;;
            @rpath/*)
                # only our own relinked references may use @rpath
                [ -e "$frameworks/${dep#@rpath/}" ] && continue
                echo "unresolved $dep in $file" >&2
                exit 1
                ;;
            *) continue ;; # system libraries
        esac
        name=$(basename "$dep")
        install_name_tool -change "$dep" "@rpath/$name" "$file" 2>/dev/null
        [ -e "$frameworks/$name" ] && continue
        real=$(realpath "$src")
        cp "$real" "$frameworks/$name"
        chmod u+w "$frameworks/$name"
        install_name_tool -id "@rpath/$name" "$frameworks/$name" 2>/dev/null
        copy_licenses "$real"
        bundle_deps "$frameworks/$name" "$(dirname "$real")"
    done
)

for exe in tbis mpv; do
    install_name_tool -add_rpath @executable_path/../Frameworks "$app/Contents/MacOS/$exe"
    bundle_deps "$app/Contents/MacOS/$exe" "$brew_prefix/opt/mpv/bin"
done
copy_licenses "$(realpath "$brew_prefix/opt/mpv/bin/mpv")"

# nothing may still point into Homebrew, or the app breaks on Macs without it
if otool -L "$app/Contents/MacOS/"* "$frameworks"/* | grep -q "$brew_prefix"; then
    otool -L "$app/Contents/MacOS/"* "$frameworks"/* | grep "$brew_prefix" >&2
    echo "bundle still references $brew_prefix" >&2
    exit 1
fi

# versions of everything bundled, for the GPL source offer (ADR-0004)
ls "$licenses" > "$app/Contents/Resources/bundled-components.txt"

iconset=target/tbis.iconset
rm -rf "$iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z $size $size assets/icon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z $double $double assets/icon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/tbis.icns"
rm -rf "$iconset"

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>tbis</string>
    <key>CFBundleDisplayName</key>
    <string>tbis</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.mahi160.tbis</string>
    <key>CFBundleExecutable</key>
    <string>tbis</string>
    <key>CFBundleIconFile</key>
    <string>tbis</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$version</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# relinking invalidated signatures; arm64 refuses to load unsigned code. Inner
# code first, then an ad-hoc signature covering Info.plist and resources
codesign --force --sign - "$frameworks"/* "$app/Contents/MacOS/mpv"
codesign --force --sign - "$app"
echo "built $app"
