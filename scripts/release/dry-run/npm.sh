#!/bin/sh
# `npm publish --dry-run` of each npm package under dist/release/packages/js:
# npm packs the tarball and checks the manifest, and uploads nothing. The script
# fails when npm prints a warning other than "requires you to be logged in" (a
# dry run always prints that one; npm warns about anything else when it corrects
# the manifest), and when a scoped package does not set `publishConfig.access`
# to public, which npm would publish as private.
#
#   scripts/release/dry-run/npm.sh <the directory of the .tgz files>
set -eu

dir=$1
count=0
for tgz in "$dir"/*.tgz; do
    [ -f "$tgz" ] || continue
    count=$((count + 1))
    log=$(mktemp)
    npm publish --dry-run --ignore-scripts "$tgz" >"$log" 2>&1 || { cat "$log" >&2; echo "dry-run npm: $tgz failed" >&2; exit 1; }
    if grep -i 'npm warn' "$log" | grep -qiv 'requires you to be logged in'; then
        cat "$log" >&2
        echo "dry-run npm: $tgz made npm warn" >&2
        exit 1
    fi
    name=$(tar -xzOf "$tgz" package/package.json | node -e '
        const m = JSON.parse(require("fs").readFileSync(0, "utf8"));
        if (m.name.startsWith("@") && !(m.publishConfig && m.publishConfig.access === "public")) {
            console.error(m.name + " is scoped and does not say publishConfig.access is public");
            process.exit(1);
        }
        console.log(m.name + "@" + m.version);
    ')
    echo "npm: $name"
done
[ "$count" -gt 0 ] || { echo "dry-run npm: no .tgz in $dir" >&2; exit 1; }
