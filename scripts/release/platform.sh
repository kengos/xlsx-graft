#!/bin/sh
# Prints one column of one platform's row in platforms.tsv.
#
#   scripts/release/platform.sh <slug> <column>      e.g. linux-x64-gnu capi
#   scripts/release/platform.sh --slugs              every slug, one per line
#
# Fails when the slug or the column is not in the table, so a typo never
# reaches a package as an empty name.
set -eu

table="$(dirname "$0")/platforms.tsv"

if [ "${1:-}" = "--slugs" ]; then
    awk -F'\t' '/^#/ { next } NR_DATA++ > 0 { print $1 }' "$table"
    exit 0
fi

if [ $# -ne 2 ]; then
    echo "usage: $0 <slug> <column> | --slugs" >&2
    exit 2
fi

awk -F'\t' -v slug="$1" -v column="$2" '
    /^#/ { next }
    !header { for (i = 1; i <= NF; i++) { if ($i == column) { index_of = i } } header = 1; next }
    $1 == slug { row = 1; if (index_of) { print $index_of; found = 1 } }
    END {
        if (!index_of) { print "platform.sh: no column " column > "/dev/stderr"; exit 1 }
        if (!row) { print "platform.sh: no platform " slug > "/dev/stderr"; exit 1 }
        if (!found) { exit 1 }
    }' "$table"
