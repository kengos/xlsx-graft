#!/bin/sh
# Builds the .NET package of a release, inside the .NET gate image (the floor
# line, mk/release.mk): ONE nupkg, whose runtimes/<rid>/native/ holds the engine
# library of each platform (NuGet copies the one that fits into the output of a
# build for it), with the license files and THIRD-PARTY-LICENSES.
#
#   scripts/release/assemble-dotnet.sh <slug>...
#
# Reads  /repo/dist/release/native/<slug>/<the C ABI library>
# Writes /repo/dist/release/packages/dotnet/
#
# Nothing here talks to a registry.
set -eu

repo=/repo
tool="$repo/scripts/release"
out="$repo/dist/release/packages/dotnet"
work=/tmp/release-dotnet

rm -rf "$out" "$work"
mkdir -p "$out"
cp -r "$repo/sdk/dotnet" "$work"
rm -rf "$work/XlsxGraft/runtimes"
(cd "$repo" && cp LICENSE-APACHE LICENSE-BSD LICENSE-MIT NOTICE THIRD-PARTY-LICENSES "$work/")

for slug in "$@"; do
    native="$work/XlsxGraft/runtimes/$("$tool/platform.sh" "$slug" rid)/native"
    mkdir -p "$native"
    cp "$repo/dist/release/native/$slug/$("$tool/platform.sh" "$slug" capi)" "$native/"
    echo "  dotnet: $slug"
done

# Packed into a directory of its own and copied: `dotnet pack` fails with "Only
# one project can be specified" when -o names a directory in the mounted
# checkout, as sdk:verify:dotnet's pack, which writes to /tmp, never does.
dotnet pack "$work/XlsxGraft/XlsxGraft.csproj" -c Release -o /tmp/release-dotnet-pack
cp /tmp/release-dotnet-pack/*.nupkg "$out/"
ls "$out"
