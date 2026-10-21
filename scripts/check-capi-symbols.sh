#!/bin/sh
# The built C ABI library exports exactly the functions its header declares,
# and nothing else.
#
#   scripts/check-capi-symbols.sh <shared library> <header>
#
# include/xlsx_graft.h is hand-written, so it can drift from the library in
# both directions: an export the header never declares is unusable, and a
# declaration with nothing behind it is a link error in someone else's build.
#
# It reads the BUILT library rather than the Rust sources: an export that
# loses `#[unsafe(no_mangle)]` still compiles, and every Rust test still
# passes through the rlib, but the symbol is gone from the shared library.
#
# Exported: every defined symbol in the dynamic symbol table
# (`nm -D --defined-only`, GNU binutils, so this runs inside the toolchain
# image). A text symbol (T) named `xlsx_graft_*` is a function to compare with
# the header. Anything else — data (D, B, R), weak or indirect symbols, or a
# name without the prefix, which can clash with a symbol in the host
# process — fails the check: the header declares only functions.
#
# Declared: a line that starts at column zero with a letter and names
# `xlsx_graft_<name>(`. Comments, preprocessor lines and the continuation
# lines of a long declaration do not.
#
# Read-only POSIX sh + nm and standard tools.
set -eu

if [ $# -ne 2 ]; then
    echo "usage: $0 <shared library> <header>" >&2
    exit 2
fi
lib=$1
header=$2
for f in "$lib" "$header"; do
    if [ ! -f "$f" ]; then
        echo "capi-symbols: $f does not exist" >&2
        exit 1
    fi
done

# Each tool that reads a file runs on its own, so that its failure stops the
# script instead of being hidden inside a pipeline.
table=$(nm -D --defined-only "$lib")
exported=$(printf '%s\n' "$table" |
    awk 'NF >= 3 && $2 == "T" && $3 ~ /^xlsx_graft_/ { print $3 }' | LC_ALL=C sort -u)
unexpected=$(printf '%s\n' "$table" |
    awk 'NF >= 3 && !($2 == "T" && $3 ~ /^xlsx_graft_/) { print $2 " " $3 }' | LC_ALL=C sort -u)
declarations=$(awk '
    { sub(/\r$/, "") } # a CRLF checkout reads the same
    /^[A-Za-z]/ && match($0, /xlsx_graft_[A-Za-z0-9_]*[ \t]*\(/) {
        name = substr($0, RSTART, RLENGTH)
        sub(/[ \t]*\($/, "", name)
        print name
    }
' "$header")
declared=$(printf '%s\n' "$declarations" | LC_ALL=C sort -u)

exported_count=$(printf '%s\n' "$exported" | grep -c . || true)
declared_count=$(printf '%s\n' "$declared" | grep -c . || true)
if [ "$exported_count" -eq 0 ]; then
    echo "capi-symbols: no xlsx_graft_* function exported by $lib — the scan is broken (or every export lost its no_mangle), not clean" >&2
    exit 1
fi
if [ "$declared_count" -eq 0 ]; then
    echo "capi-symbols: no xlsx_graft_* function declared in $header — the scan is broken, not clean" >&2
    exit 1
fi

undeclared=$(printf '%s\n' "$exported" | grep -vxF -e "$declared" || true)
unexported=$(printf '%s\n' "$declared" | grep -vxF -e "$exported" || true)
if [ -n "$unexpected$undeclared$unexported" ]; then
    if [ -n "$unexpected" ]; then
        printf '%s\n' "$unexpected" | sed "s|^|capi-symbols: exported but not an xlsx_graft_* function (nm type, name): |" >&2
    fi
    if [ -n "$undeclared" ]; then
        printf '%s\n' "$undeclared" | sed "s|^|capi-symbols: exported but not declared in $header: |" >&2
    fi
    if [ -n "$unexported" ]; then
        printf '%s\n' "$unexported" | sed "s|^|capi-symbols: declared but not exported by $lib: |" >&2
    fi
    exit 1
fi
echo "capi-symbols: OK ($exported_count functions)"
