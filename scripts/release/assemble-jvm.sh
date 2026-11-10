#!/bin/sh
# Builds the Java packages of a release, inside the Java gate image (the floor
# line, mk/release.mk): the jar of the classes (no engine), and a jar of each
# platform with the engine library in native/<os>-<arch>/ (the directory JNA
# names), published to a Maven repository of its own in the layout Maven
# Central takes, with the sources and javadoc jars and the POM.
#
#   scripts/release/assemble-jvm.sh <slug>...
#
# Reads  /repo/dist/release/native/<slug>/<the C ABI library>
# Writes /repo/dist/release/packages/jvm/
#
# Nothing here talks to a registry.
set -eu

repo=/repo
tool="$repo/scripts/release"
out="$repo/dist/release/packages/jvm"
work=/tmp/release-jvm

rm -rf "$out" "$work"
mkdir -p "$out" "$work/natives"
cp -r "$repo/sdk/jvm" "$work/build"

for slug in "$@"; do
    dir="$work/natives/$("$tool/platform.sh" "$slug" jna)"
    mkdir -p "$dir"
    cp "$repo/dist/release/native/$slug/$("$tool/platform.sh" "$slug" capi)" "$dir/"
    cp "$repo/THIRD-PARTY-LICENSES" "$dir/"
    echo "  jvm: $slug"
done

cd "$work/build"
XLSX_GRAFT_NOTICE_DIR="$repo" XLSX_GRAFT_NATIVE_DIR="$work/natives" \
    gradle --offline publishToMavenLocal -Dmaven.repo.local="$out"
find "$out" -name '*.jar' | sort
