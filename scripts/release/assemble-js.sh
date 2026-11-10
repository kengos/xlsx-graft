#!/bin/sh
# Builds the npm packages of a release, inside the JS gate image (the floor
# line, mk/release.mk): the package of the classes, which names the platform
# packages as its optional dependencies, and a package of each platform with
# the addon (xlsx_graft.node) in it.
#
#   scripts/release/assemble-js.sh <slug>...
#
# Reads  /repo/dist/release/native/<slug>/xlsx_graft.node
# Writes /repo/dist/release/packages/js/
#
# The optional dependencies are written into the package.json of the build
# copy only, after the build: the names resolve nowhere until a release
# publishes them, and a checkout's frozen lockfile would refuse them. Nothing
# here talks to a registry.
set -eu

repo=/repo
tool="$repo/scripts/release"
out="$repo/dist/release/packages/js"
work=/tmp/release-js

rm -rf "$out" "$work"
mkdir -p "$out" "$work"
cp -r "$repo/sdk/js" "$work/build"
cd "$work/build"
rm -rf node_modules dist coverage native
cp "$repo/LICENSE-APACHE" "$repo/LICENSE-BSD" "$repo/LICENSE-MIT" "$repo/NOTICE" .
pnpm install --offline --frozen-lockfile
pnpm run build
version=$(node -p 'require("./package.json").version')

# One package a platform. os and cpu are the first two words of the slug; npm
# skips a package whose os, cpu or libc does not fit, which is how only the
# right one is installed.
names=""
for slug in "$@"; do
    name=$("$tool/platform.sh" "$slug" npm)
    names="$names $name"
    pkg="$work/$slug"
    mkdir -p "$pkg"
    cp "$repo/dist/release/native/$slug/xlsx_graft.node" "$pkg/"
    cp "$repo/LICENSE-APACHE" "$repo/LICENSE-BSD" "$repo/LICENSE-MIT" "$repo/NOTICE" \
        "$repo/THIRD-PARTY-LICENSES" "$pkg/"
    PKG_NAME="$name" PKG_SLUG="$slug" PKG_VERSION="$version" node -e '
        const [os, cpu] = process.env.PKG_SLUG.split("-");
        const manifest = {
            name: process.env.PKG_NAME,
            version: process.env.PKG_VERSION,
            description: "The xlsx-graft engine as a Node.js addon, for " + process.env.PKG_SLUG,
            license: "MIT OR Apache-2.0 OR BSD-3-Clause",
            repository: { type: "git", url: "git+https://github.com/kengos/xlsx-graft.git" },
            files: ["xlsx_graft.node", "LICENSE-*", "NOTICE", "THIRD-PARTY-LICENSES"],
            // A scoped package is published as private unless it says so.
            publishConfig: { access: "public" },
            os: [os],
            cpu: [cpu],
            ...(os === "linux" ? { libc: ["glibc"] } : {}),
        };
        require("fs").writeFileSync(process.argv[1], JSON.stringify(manifest, null, 2));
    ' "$pkg/package.json"
    (cd "$pkg" && npm pack --pack-destination "$out" >/dev/null)
    echo "  js: $slug"
done

# The package of the classes, with the platform packages as optional
# dependencies at its own version.
OPTIONAL="$names" node -e '
    const fs = require("fs");
    const manifest = JSON.parse(fs.readFileSync("package.json", "utf8"));
    manifest.optionalDependencies = Object.fromEntries(
        process.env.OPTIONAL.split(" ").filter(Boolean).map((name) => [name, manifest.version]),
    );
    fs.writeFileSync("package.json", JSON.stringify(manifest, null, 2));
'
pnpm pack --pack-destination "$out" >/dev/null
echo "  js: package of the classes"
ls "$out"
