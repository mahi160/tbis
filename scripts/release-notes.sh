#!/bin/sh
# Prints markdown release notes from conventional commits since the last `v*` tag
# (all history before the first release): feat, fix and perf subjects, grouped.
# Run before tagging the new release, like scripts/next-version.sh.
set -eu
cd "$(dirname "$0")/.."

if last=$(git describe --tags --abbrev=0 --match 'v*' 2>/dev/null); then
    range="$last..HEAD"
else
    range=HEAD
fi

# `## <heading>` then one bullet per `<type>(scope)!: subject` commit, scope in bold
section() {
    lines=$(git log --no-merges --format=%s "$range" |
        sed -n -E -e "s/^$1\(([^)]*)\)!?: (.*)$/- **\1:** \2/p" \
            -e "s/^$1!?: (.*)$/- \1/p")
    if [ -n "$lines" ]; then
        printf '## %s\n\n%s\n\n' "$2" "$lines"
    fi
}

section feat Features
section fix Fixes
section perf Performance
