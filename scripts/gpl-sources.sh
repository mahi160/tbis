#!/bin/sh
# Writes the GPL Corresponding Source (ADR-0004) for a built target/tbis.app to
# the tar file `$1`: tbis at HEAD, plus the source archive and Homebrew formula
# (its build recipe and patches) of each component listed in the bundle.
# Run in the same environment as scripts/bundle.sh, so the Homebrew formula
# versions match what was bundled.
set -eu
cd "$(dirname "$0")/.."

out=$(realpath "$(dirname "$1")")/$(basename "$1")
list=target/tbis.app/Contents/Resources/bundled-components.txt
work=$(mktemp -d)
dir="$work/sources"
mkdir -p "$dir/formulae"

git archive --format=tar.gz --prefix=tbis/ -o "$dir/tbis.tar.gz" HEAD

while read -r component; do
    formula=${component%-*} # "<formula>-<version>"; versions contain no '-'
    current=$(brew info --json=v1 "$formula" |
        jq -r '.[0] | .versions.stable + (if .revision > 0 then "_\(.revision)" else "" end)')
    if [ "$formula-$current" != "$component" ]; then
        echo "bundled $component but the formula is now $current; rebuild the bundle first" >&2
        exit 1
    fi
    brew fetch --quiet --build-from-source "$formula"
    cp "$(brew --cache --build-from-source "$formula")" "$dir/"
    brew cat "$formula" > "$dir/formulae/$formula.rb"
done < "$list"

tar -cf "$out" -C "$work" sources
rm -rf "$work"
