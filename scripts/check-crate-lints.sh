#!/bin/sh
# Every engine crate opts into the workspace lints.
#
# `[workspace.lints]` in engine/Cargo.toml applies only to a crate whose own
# Cargo.toml says
#
#     [lints]
#     workspace = true
#
# A crate without it gets none of them — no panic lints, no mod.rs refusal, no
# unsafe_code denial — and clippy stays green, so nothing else would notice.
#
# The crates are found from the directories, not from the workspace `members`
# list, so a crate missing from that list is checked too. Only the spelling
# above is accepted: TOML has others (`lints.workspace = true`, an inline
# table), and telling them apart in shell is where a check like this goes wrong.
#
# Pure read-only POSIX sh + standard tools; no toolchain required.
set -eu

root=$(dirname "$0")/..

# Prune only cargo's output directory, which holds its own Cargo.toml files.
manifests=$(find "$root/engine" -path "$root/engine/target" -prune -o -name Cargo.toml -print |
    grep -vxF "$root/engine/Cargo.toml" | LC_ALL=C sort)
count=$(printf '%s\n' "$manifests" | grep -c . || true)
if [ "$count" -eq 0 ]; then
    echo "crate-lints: no crate Cargo.toml found — the scan is broken, not clean" >&2
    exit 1
fi

violations=$(
    printf '%s\n' "$manifests" |
    while IFS= read -r f; do
        if ! awk '
            { sub(/\r$/, "") } # a CRLF checkout reads the same
            /^[ \t]*\[/ { section = $0; sub(/[ \t]*(#.*)?$/, "", section); sub(/^[ \t]*/, "", section) }
            section == "[lints]" && /^[ \t]*workspace[ \t]*=[ \t]*true[ \t]*(#.*)?$/ { found = 1 }
            END { exit !found }
        ' "$f"; then
            echo "${f#"$root/"} does not opt into the workspace lints (add exactly: [lints] / workspace = true)"
        fi
    done
)

if [ -n "$violations" ]; then
    printf '%s\n' "$violations" | sed 's/^/crate-lints: /' >&2
    exit 1
fi
echo "crate-lints: OK ($count crates)"
