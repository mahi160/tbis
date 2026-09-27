#!/bin/sh
# Prints the next release version from conventional commits since the last `v*` tag:
# breaking change = major, any feat = minor, otherwise patch. Before the first
# release there is no tag to bump from, so Cargo.toml's version is used as is.
set -eu
cd "$(dirname "$0")/.."

if ! last=$(git describe --tags --abbrev=0 --match 'v*' 2>/dev/null); then
    sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1
    exit 0
fi
current=${last#v}
range="$last..HEAD"

subjects=$(git log --format=%s "$range")
bodies=$(git log --format=%b "$range")

IFS=. read -r major minor patch <<EOF
$current
EOF

if printf '%s\n' "$subjects" | grep -Eq '^[a-z]+(\([^)]*\))?!:' ||
    printf '%s\n' "$bodies" | grep -q '^BREAKING CHANGE'; then
    echo "$((major + 1)).0.0"
elif printf '%s\n' "$subjects" | grep -Eq '^feat(\([^)]*\))?:'; then
    echo "$major.$((minor + 1)).0"
else
    echo "$major.$minor.$((patch + 1))"
fi
