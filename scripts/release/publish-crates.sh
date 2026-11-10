#!/bin/sh
# Publishes the crates of scripts/release/crates.txt to crates.io, one at a
# time, in that order, from the engine/ workspace (run from the repository
# root, on a checkout of the release's tag).
#
#   scripts/release/publish-crates.sh [--dry-run]
#
# A crate whose version is already on crates.io is skipped, so that the run
# can be started again after a failure part of the way through: `cargo publish
# --workspace` would stop at the first crate it finds published. With
# FAIL_IF_PUBLISHED=true a published crate fails the run instead. The token is
# CARGO_REGISTRY_TOKEN (the OIDC exchange of the workflow). With --dry-run no
# crate is published and no token is needed: each crate's state is printed and
# `cargo publish --workspace --dry-run` runs.
set -eu

root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root/engine"

dry=false
[ "${1:-}" = "--dry-run" ] && dry=true
fail_if_published=${FAIL_IF_PUBLISHED:-false}

version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"$/\1/p' Cargo.toml)
[ -n "$version" ] || { echo "publish-crates: no [workspace.package] version in engine/Cargo.toml" >&2; exit 1; }

# The lines of crates.txt that are not comments, without blanks: the same
# reading check_packages.py --crates makes of it.
crates=$(grep -v '^[[:space:]]*#' ../scripts/release/crates.txt | tr -d '\r' | sed 's/[[:space:]]//g' | grep . || true)
[ -n "$crates" ] || { echo "publish-crates: no crate in scripts/release/crates.txt" >&2; exit 1; }

state() { # crate -> published | missing
    code=$(curl -sS -o /dev/null -w '%{http_code}' \
        -A 'xlsx-graft-release (https://github.com/kengos/xlsx-graft)' \
        "https://crates.io/api/v1/crates/$1/$version")
    case "$code" in
        200) echo published ;;
        404) echo missing ;;
        *) echo "publish-crates: crates.io answered HTTP $code for $1 $version" >&2; return 1 ;;
    esac
}

for crate in $crates; do
    have=$(state "$crate")
    if [ "$have" = published ]; then
        if [ "$fail_if_published" = true ]; then
            echo "publish-crates: $crate $version is already on crates.io" >&2
            exit 1
        fi
        echo "$crate $version: already on crates.io, skipped"
    elif [ "$dry" = true ]; then
        echo "$crate $version: not on crates.io, would be published"
    else
        echo "$crate $version: publishing"
        cargo publish -p "$crate" --locked
    fi
done

if [ "$dry" = true ]; then
    cargo publish --workspace --dry-run --locked
fi
