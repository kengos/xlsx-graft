#!/bin/sh
# Installs the npm package of the classes and the package of the host's platform
# into a project of its own (the other platforms' packages are optional, and
# left out) and loads it with no XLSX_GRAFT_ADDON, so the only engine it can
# reach is the addon in the platform's package. Runs inside the JS gate image.
#
#   scripts/release/smoke-js.sh <slug> <engine version>
set -eu

slug=$1
want=$2
packages=/repo/dist/release/packages/js
platform=$(/repo/scripts/release/platform.sh "$slug" npm | sed 's/^@//; s|/|-|')

unset XLSX_GRAFT_ADDON
mkdir /tmp/user
cd /tmp/user
echo '{"private":true,"type":"module"}' > package.json
npm install --offline --ignore-scripts --omit=optional \
    "$packages"/xlsx-graft-[0-9]*.tgz "$packages/$platform"-[0-9]*.tgz
node --input-type=module -e '
    import { version } from "xlsx-graft";
    const result = version();
    if (!result.success || result.value !== process.argv[1]) {
        console.error(`engine is ${result.value}, not ${process.argv[1]}`);
        process.exit(1);
    }
    console.log(`platform package loads engine ${result.value}`);
' "$want"
