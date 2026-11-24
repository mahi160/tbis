#!/bin/sh
set -eu

if [ "$(uname -sm)" != "Linux x86_64" ] || [ "$#" -ne 2 ]; then
    echo "Usage on x86-64 Linux: sh scripts/build-flatpak-candidate.sh REVISION OUTPUT_DIR" >&2
    exit 2
fi

revision=$(git rev-parse --verify "$1^{commit}")
mkdir -p "$2"
output=$(cd "$2" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
trap 'exit 1' HUP INT TERM

git archive --format=tar -o "$work/source.tar" "$revision"
cd "$work"
mkdir source
tar -xf source.tar -C source

id=io.github.mahi160.tbis
manifest="$id.yml"
cp "source/packaging/flatpak/$manifest" .
(
    cd source
    cargo vendor --locked cargo-vendor > ../cargo-sources.toml
    printf '\n' >> .cargo/config.toml
    cat ../cargo-sources.toml >> .cargo/config.toml
)

{
    printf 'Source revision: %s\n' "$revision"
    for ref in org.freedesktop.Platform org.freedesktop.Sdk org.freedesktop.Sdk.Extension.rust-stable; do
        commit=$(flatpak info --user --show-commit "$ref/x86_64/26.08")
        printf '%s/x86_64/26.08 %s\n' "$ref" "$commit"
    done
} > provenance.txt

flatpak-builder --user --arch=x86_64 --default-branch=candidate --repo=repo build "$manifest"
bundle=tbis-linux-x86_64.flatpak
flatpak build-bundle --arch=x86_64 --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo \
    repo "$bundle" "$id" candidate
sha256sum "$bundle" > "$bundle.sha256"
cp "$bundle" "$bundle.sha256" provenance.txt "$output/"
