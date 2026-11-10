#!/bin/sh
# Builds the Python wheels of a release, inside the Python gate image (the
# floor line, mk/release.mk): one wheel with no engine in it (py3-none-any),
# and a platform wheel per platform with the engine library in
# xlsx_graft/native/.
#
#   scripts/release/assemble-python.sh <slug>...
#
# Reads  /repo/dist/release/native/<slug>/<the C ABI library>
# Writes /repo/dist/release/packages/python/
#
# The wheel with no engine is what pip installs where no platform wheel fits;
# importing it and calling the engine raises the error that says where to point
# XLSX_GRAFT_LIBRARY. Built with the locked hatchling and no isolation, as
# sdk:verify:python builds the wheel, so nothing is downloaded. Nothing here
# talks to a registry.
set -eu

repo=/repo
tool="$repo/scripts/release"
out="$repo/dist/release/packages/python"
work=/tmp/release-python

rm -rf "$out" "$work"
mkdir -p "$out" "$work"

build_wheel() { # name platform-tag slug
    stage="$work/$1"
    cp -r "$repo/sdk/python" "$stage"
    rm -rf "$stage/src/xlsx_graft/native"
    (cd "$repo" && cp LICENSE-APACHE LICENSE-BSD LICENSE-MIT NOTICE "$stage/")
    if [ -n "$3" ]; then
        mkdir -p "$stage/src/xlsx_graft/native"
        cp "$repo/dist/release/native/$3/$("$tool/platform.sh" "$3" capi)" "$stage/src/xlsx_graft/native/"
        cp "$repo/THIRD-PARTY-LICENSES" "$stage/src/xlsx_graft/native/"
    fi
    (cd "$stage" && XLSX_GRAFT_WHEEL_PLATFORM="$2" python -m build --wheel --no-isolation \
        --outdir "$stage/dist" . >/dev/null && cp "$stage"/dist/*.whl "$out/")
}

build_wheel any "" ""
echo "  python: any wheel"
for slug in "$@"; do
    build_wheel "$slug" "$("$tool/platform.sh" "$slug" wheel)" "$slug"
    echo "  python: $slug"
done
ls "$out"
