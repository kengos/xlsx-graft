#!/bin/sh
# Every NOTICE file of a dependency is in THIRD-PARTY-LICENSES.
#
#   scripts/check-notice-files.sh <list>      (run in engine/)
#
# <list> is the text cargo-about wrote (engine/about.hbs). Apache-2.0 section
# 4(d) makes the NOTICE file of a dependency part of what is distributed with
# it, and cargo-about does not say what it treats as a license file. This
# finds every file whose name starts with NOTICE (any case) in the top
# directory of each crate listed under "Used by:", and fails for a file whose
# first non-blank line is not a whole line of <list>. It does not compare the
# files' other lines.
#
# The crates are in $CARGO_HOME/registry/src, where `cargo fetch` puts them
# (engine:licenses and engine:licenses-write in mk/engine/licenses.mk run it
# first). A listed crate that is not there fails, so that a wrong registry
# directory is not read as "no NOTICE files". The workspace's own crates
# (xlsx-graft-*) are not looked for.
#
# POSIX sh + sed + grep + find; runs inside the toolchain image.
set -eu

if [ $# -ne 1 ]; then
    echo "usage: $0 <list>" >&2
    exit 2
fi
list=$1
registry=${CARGO_HOME:-$HOME/.cargo}/registry/src

status=0
count=0
seen=0
# "- name 1.2.3" lines: the crates under "Used by:" (the license summary lines
# have a colon and are not matched).
crates=$(sed -n 's/^- \([^ :]*\) \([0-9][^ ]*\)$/\1-\2/p' "$list" | LC_ALL=C sort -u)
for crate in $crates; do
    # xlsx-graft's own crates are listed too (engine/about.toml), but they are
    # not downloaded from a registry, so $registry holds no copy of them to
    # look in (a publish dry run may leave one, which is not read either), and
    # their NOTICE is the project's, which is in the repository root.
    case "$crate" in xlsx-graft-*) continue ;; esac
    found=0
    for dir in "$registry"/*/"$crate"; do
        [ -d "$dir" ] || continue
        found=1
        seen=$((seen + 1))
        for file in $(find "$dir" -maxdepth 1 -type f -iname 'NOTICE*' | LC_ALL=C sort); do
            count=$((count + 1))
            first=$(sed -n '/[^[:space:]]/{p;q;}' "$file")
            if ! grep -qxF -- "$first" "$list"; then
                echo "notice: $file is not in $list; add it as a license file of the crate with a [<crate>.clarify] entry in engine/about.toml (cargo-about's documentation, Clarify), then run make engine:licenses-write" >&2
                status=1
            fi
        done
    done
    if [ "$found" -eq 0 ]; then
        echo "notice: $crate is listed in $list but is not in $registry" >&2
        status=1
    fi
done
[ "$status" -eq 0 ] || exit 1
echo "notice: OK ($count NOTICE file(s) in $seen crate(s); all are in the list)"
