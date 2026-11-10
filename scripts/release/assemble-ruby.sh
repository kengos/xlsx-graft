#!/bin/sh
# Builds the Ruby gems of a release, inside the Ruby gate image (the floor
# line, mk/release.mk): one gem with no engine in it, and a platform gem per
# platform with the engine library in lib/xlsx_graft/native/.
#
#   scripts/release/assemble-ruby.sh <slug>...
#
# Reads  /repo/dist/release/native/<slug>/<the C ABI library>
# Writes /repo/dist/release/packages/ruby/
#
# The gem with no engine is what RubyGems installs where no platform gem fits
# (musl Linux, Windows arm64, ...); loading it raises the error that says where
# to point XLSX_GRAFT_LIBRARY. Nothing here talks to a registry.
set -eu

repo=/repo
tool="$repo/scripts/release"
out="$repo/dist/release/packages/ruby"
work=/tmp/release-ruby

rm -rf "$out" "$work"
mkdir -p "$out" "$work"

# A copy of the SDK the way a checkout has it, plus what a package carries
# from the repository's root. An empty slug builds the gem with no engine.
build_gem() { # name platform slug
    stage="$work/$1"
    cp -r "$repo/sdk/ruby" "$stage"
    rm -rf "$stage/coverage" "$stage/lib/xlsx_graft/native"
    (cd "$repo" && cp LICENSE-APACHE LICENSE-BSD LICENSE-MIT NOTICE "$stage/")
    if [ -n "$3" ]; then
        mkdir -p "$stage/lib/xlsx_graft/native"
        cp "$repo/dist/release/native/$3/$("$tool/platform.sh" "$3" capi)" "$stage/lib/xlsx_graft/native/"
        cp "$repo/THIRD-PARTY-LICENSES" "$stage/"
    fi
    (cd "$stage" && XLSX_GRAFT_GEM_PLATFORM="$2" gem build xlsx_graft.gemspec >/dev/null \
        && cp xlsx_graft-*.gem "$out/")
}

build_gem plain "" ""
echo "  ruby: plain gem"
for slug in "$@"; do
    build_gem "$slug" "$("$tool/platform.sh" "$slug" gem)" "$slug"
    echo "  ruby: $slug"
done
ls "$out"
