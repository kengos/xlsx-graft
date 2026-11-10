#!/bin/sh
# Resolves the jar of the classes and the jar of the host's platform from the
# Maven repository of a release and runs them with no XLSX_GRAFT_LIBRARY, so the
# only engine they can reach is the library in the platform's jar. Runs inside
# the Java gate image.
#
#   scripts/release/smoke-jvm.sh <slug>
set -eu

unset XLSX_GRAFT_LIBRARY
cp -r /repo/dist/release/packages/jvm /tmp/repo
cp -r /repo/sdk/jvm/script/check-installed-jar /tmp/user
cd /tmp/user
XLSX_GRAFT_CLASSIFIER=$(/repo/scripts/release/platform.sh "$1" jna) \
    gradle --offline run --args="/tmp/repo /repo/sdk/jvm/gradle.lockfile /repo/engine/Cargo.toml"
