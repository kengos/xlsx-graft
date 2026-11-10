#!/bin/sh
# Installs the platform wheel of the Docker host's platform into a virtual
# environment of its own and imports it with no XLSX_GRAFT_LIBRARY, so the only
# engine it can reach is the library the wheel carries; pip refuses a wheel
# whose platform tag does not fit this machine. Then runs the CLI of the same
# platform from its archive. Runs inside the Python gate image.
#
#   scripts/release/smoke-python.sh <slug> <engine version>
set -eu

slug=$1
want=$2
tool=/repo/scripts/release
packages=/repo/dist/release/packages
wheel=$(ls "$packages/python/xlsx_graft-"*"-py3-none-$("$tool/platform.sh" "$slug" wheel).whl")

unset XLSX_GRAFT_LIBRARY
/usr/local/bin/python3 -m venv /tmp/user
/tmp/user/bin/python -m pip install --no-index --no-deps --no-cache-dir "$wheel"
cd /tmp
/tmp/user/bin/python -I -c '
import sys
import xlsx_graft

result = xlsx_graft.version()
if not result.success or result.value != sys.argv[1]:
    sys.exit(f"engine is {result.value}, not {sys.argv[1]}: {result.diagnostics}")
print(f"platform wheel loads engine {result.value}")
' "$want"

mkdir /tmp/cli
tar -xzf "$packages/cli/xlsx-graft-$want-$slug.tar.gz" -C /tmp/cli
json=$(/tmp/cli/xlsx-graft --version --format json)
case "$json" in
    *"\"version\":\"$want\""*) echo "cli prints $json" ;;
    *) echo "smoke: the CLI printed $json, not version $want" >&2; exit 1 ;;
esac
