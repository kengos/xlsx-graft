#!/bin/sh
# Line-budget gate for engine Rust sources:
#
#   - every non-test .rs file under engine/ is <= 300 lines; <= 160 is the
#     design target for a NEW file, and is not gated,
#   - every .rs file starts with a `//!` role header so `head -1` / grep
#     can identify or skip it without opening,
#   - a file may exceed 300 lines only with an explicit in-file waiver:
#     a line containing `line-budget-exempt: <reason>`.
#
# Hitting the cap is a signal about the design, not an instruction to trim:
# split the file at a seam that means something.
#
# TEST FILES ARE OUT OF SCOPE for the length half: a test file is a list of
# independent cases rather than a unit of design, and splitting a suite to fit
# a number moves cases across a seam chosen by arithmetic. The //! header still
# applies to every file — that one is about finding a file, not its shape.
#
# Pure read-only POSIX sh + standard tools; no toolchain required.
set -eu

HARD_MAX=300
root=$(dirname "$0")/..

# `<mod>/tests.rs`, anything under a `tests/` directory (both the sibling
# `<mod>/tests/` split and a crate's near-e2e `tests/` binary), and the
# `<mod>/<feature>_tests.rs` shape.
is_test() {
    case $1 in
    */tests/* | */tests.rs | *_tests.rs) return 0 ;;
    *) return 1 ;;
    esac
}

# Prune only cargo's output directory: a source module named `target` must
# still be checked.
files=$(find "$root/engine" -path "$root/engine/target" -prune -o -name '*.rs' -print | LC_ALL=C sort)
count=$(printf '%s\n' "$files" | grep -c . || true)
if [ "$count" -eq 0 ]; then
    echo "line-budget: no .rs files found — the scan is broken, not clean" >&2
    exit 1
fi

violations=$(
    printf '%s\n' "$files" |
    while IFS= read -r f; do
        rel=${f#"$root/"}
        # BSD wc (macOS) pads the count with spaces; GNU wc does not.
        lines=$(wc -l < "$f" | tr -d ' ')
        # Match on the repo-relative path: a checkout that itself sits under a
        # `tests/` directory would otherwise make every file look like a test.
        if ! is_test "$rel" && [ "$lines" -gt "$HARD_MAX" ] &&
            ! grep -q 'line-budget-exempt:' "$f"; then
            echo "$rel is $lines lines (max $HARD_MAX; split it at a seam that means something, or add 'line-budget-exempt: <reason>')"
        fi
        case $(head -1 "$f") in
            ("//!"*) ;;
            (*) echo "$rel is missing a first-line //! role header" ;;
        esac
    done
)

if [ -n "$violations" ]; then
    printf '%s\n' "$violations" | sed 's/^/line-budget: /' >&2
    exit 1
fi
echo "line-budget: OK ($count files)"
