#!/bin/sh
# Prints the next release version from conventional commits since the last `v*` tag
# (or all commits, from Cargo.toml's version, before the first release):
# breaking change = major, any feat = minor, otherwise patch.
set -eu
cd "$(dirname "$0")/.."

if last=$(git describe --tags --abbrev=0 --match 'v*' 2>/dev/null); then
    current=${last#v}
    range="$last..HEAD"
else
    current=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
    range=HEAD
fi

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
