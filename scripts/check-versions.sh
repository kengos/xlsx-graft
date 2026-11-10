#!/bin/sh
# Fails when a package manifest's version is not the workspace's
# ([workspace.package] version in engine/Cargo.toml).
#
# Checked: the xlsx-graft crates' versions in engine/Cargo.toml (the one every
# crate's path dependency names) and the Ruby, Python, .NET, Java and JS
# manifests. PHP and Go have no version of their own: Packagist and the Go
# proxy take it from the tag. The projects that install a built package
# (sdk/dotnet/script/CheckInstalledPackage, sdk/jvm/script/check-installed-jar)
# name the version too, but sdk:verify:* fails when they are stale, so they are
# not read here.
#
# Each file has the number of versions it must hold. A pattern that no longer
# matches (a manifest reformatted) holds none, which fails instead of passing
# over a file it cannot read.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
want=$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version = "\(.*\)"$/\1/p' "$root/engine/Cargo.toml")
if [ -z "$want" ]; then
    echo "version:check: no [workspace.package] version in engine/Cargo.toml" >&2
    exit 1
fi

failed=0
# check <file> <sed pattern whose \1 is the version> <how many versions the file holds>
check() {
    values=$(sed -n "$2" "$root/$1")
    found=$(printf '%s\n' "$values" | grep -c . || true)
    if [ "$found" -ne "$3" ]; then
        echo "version:check: $1 names $found versions, not $3" >&2
        failed=1
    fi
    for value in $values; do
        if [ "$value" != "$want" ]; then
            echo "version:check: $1 says $value, engine/Cargo.toml says $want" >&2
            failed=1
        fi
    done
}

# Counted by the name alone, not by the shape `check` reads, so that a
# reformatted declaration (keys reordered, a table) fails instead of matching
# nothing in both places.
crates=$(grep -c '^xlsx-graft-' "$root/engine/Cargo.toml" || true)
if [ "$crates" -lt 1 ]; then
    echo "version:check: no xlsx-graft-* dependency in engine/Cargo.toml" >&2
    exit 1
fi
check engine/Cargo.toml 's/^xlsx-graft-[a-z-]* = { path = "[^"]*", version = "\([^"]*\)" }$/\1/p' "$crates"
check sdk/ruby/lib/xlsx_graft/version.rb 's/^ *VERSION = "\([^"]*\)"$/\1/p' 1
check sdk/python/src/xlsx_graft/_version.py 's/^__version__ = "\([^"]*\)"$/\1/p' 1
check sdk/dotnet/XlsxGraft/XlsxGraft.csproj 's/^ *<Version>\([^<]*\)<\/Version>$/\1/p' 1
check sdk/jvm/gradle.properties 's/^version=\(.*\)$/\1/p' 1
check sdk/js/package.json 's/^  "version": "\([^"]*\)",$/\1/p' 1

[ "$failed" -eq 0 ] || exit 1
echo "version: $want in the workspace, $crates path dependencies and 5 SDK manifests"
