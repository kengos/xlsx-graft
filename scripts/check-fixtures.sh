#!/bin/sh
# The workbooks in fixtures/ match the rows of fixtures/README.md, and none
# contains the account names or home folder paths Excel saves in a workbook.
#
# fixtures/README.md lists each workbook in one row of its table, the row's
# first cell the file name in backticks:
#
#     | `table.xlsx` | ... |
#
# fixtures/ holds README.md, the AppleScripts that made the workbooks
# (fixtures/scripts/*.applescript) and the workbooks, each a .xlsx or .xlsm
# file directly in fixtures/; any other file there, Finder's .DS_Store
# aside, fails the check, so a workbook cannot pass unchecked under another
# extension or in a subfolder.
# Every workbook has a row, and every row names a workbook there. Each
# workbook, as scripts/fixtures-scrub leaves it:
#
#   - docProps/core.xml has no text in dc:creator or cp:lastModifiedBy, and
#     docProps/app.xml none in Company or Manager;
#   - no comments part (xl/comments*) holds an `<author>` with text, and
#     no person part (xl/persons/*) a displayName or userId, which name the
#     accounts that wrote comments;
#   - no entry holds `absPath` (x15ac:absPath, the folder Excel saved the
#     workbook in) or a macOS or Windows home folder (`/Users/`, `:\Users\`
#     in any letter case), which Excel also writes as the absolute path of
#     a linked workbook. README.md and the AppleScripts are checked for the
#     home folder too.
#
# scripts/fixtures-scrub empties the core.xml fields and removes absPath and
# a linked workbook's absolute path; run `make fixtures:scrub`. The other
# fields it does not change, so a workbook that has them is made again
# without them.
#
# POSIX sh with the find, grep and unzip of macOS and Linux; no toolchain
# required.
set -eu

root=$(dirname "$0")/..
dir="$root/fixtures"
readme="$dir/README.md"

if [ ! -f "$readme" ]; then
    echo "fixtures: $readme is missing" >&2
    exit 1
fi

files=$(find "$dir" -maxdepth 1 -type f \( -name '*.xlsx' -o -name '*.xlsm' \) |
    sed 's|.*/||' | LC_ALL=C sort)
listed=$(sed -n 's/^| *`\([^`]*\.xls[xm]\)` *|.*/\1/p' "$readme" | LC_ALL=C sort)
if [ -z "$files" ]; then
    echo "fixtures: find listed no workbook in $dir, so every check below would pass without reading one" >&2
    exit 1
fi

# Whether the entries of the zip $1 whose names match $2 (an unzip
# wildcard, `*` for every entry) hold what the grep options and patterns
# after them say. An entry pattern no entry matches holds nothing.
holds() {
    book=$1
    entries=$2
    shift 2
    { unzip -p "$book" "$entries" 2>/dev/null || true; } | LC_ALL=C grep -aq "$@"
}

violations=$(
    find "$dir" -type f | sed "s|^$dir/||" | LC_ALL=C sort | while IFS= read -r f; do
        case $f in
        (.DS_Store | */.DS_Store) ;; # Finder's, untracked (.gitignore)
        (README.md | scripts/*.applescript)
            if LC_ALL=C grep -qiF -e '/Users/' -e ':\Users\' "$dir/$f"; then
                printf '%s\n' "$f holds a home folder path (/Users/ or :\\Users\\)"
            fi
            ;;
        (*/*) printf '%s\n' "$f is neither README.md, an AppleScript in scripts/ nor a workbook directly in fixtures/" ;;
        (*.xlsx | *.xlsm) ;;
        (*) printf '%s\n' "$f is neither README.md, an AppleScript in scripts/ nor a .xlsx or .xlsm workbook" ;;
        esac
    done
    printf '%s\n' "$files" | while IFS= read -r f; do
        printf '%s\n' "$listed" | grep -qxF "$f" ||
            printf '%s\n' "$f has no row in fixtures/README.md"
    done
    printf '%s\n' "$listed" | grep . | uniq -d | while IFS= read -r f; do
        printf '%s\n' "$f has more than one row in fixtures/README.md"
    done
    printf '%s\n' "$listed" | grep . | while IFS= read -r f; do
        printf '%s\n' "$files" | grep -qxF "$f" ||
            printf '%s\n' "fixtures/README.md lists $f, which is not in fixtures/"
    done
    printf '%s\n' "$files" | while IFS= read -r f; do
        book="$dir/$f"
        if ! unzip -tqq "$book" >/dev/null 2>&1; then
            printf '%s\n' "$f is not a zip unzip can read"
            continue
        fi
        core=$(unzip -p "$book" docProps/core.xml 2>/dev/null || true)
        app=$(unzip -p "$book" docProps/app.xml 2>/dev/null || true)
        for field in dc:creator cp:lastModifiedBy; do
            if printf '%s' "$core" | grep -q "<$field\\( [^>]*\\)\\{0,1\\}>[^<]"; then
                printf '%s\n' "$f: docProps/core.xml has text in $field"
            fi
        done
        for field in Company Manager; do
            if printf '%s' "$app" | grep -q "<$field\\( [^>]*\\)\\{0,1\\}>[^<]"; then
                printf '%s\n' "$f: docProps/app.xml has text in $field"
            fi
        done
        if holds "$book" 'xl/comments*' -e '<author>[^<]' ||
            holds "$book" 'xl/persons/*' -e 'displayName="[^"]' -e 'userId="[^"]'; then
            printf '%s\n' "$f holds a comment author or a person's displayName or userId"
        fi
        if holds "$book" '*' -F 'absPath'; then
            printf '%s\n' "$f holds an absPath"
        fi
        if holds "$book" '*' -iF -e '/Users/' -e ':\Users\'; then
            printf '%s\n' "$f holds a home folder path (/Users/ or :\\Users\\)"
        fi
    done
)

if [ -n "$violations" ]; then
    printf '%s\n' "$violations" | sed 's/^/fixtures: /' >&2
    echo "fixtures: make fixtures:scrub F=<workbook> removes the names and paths in core.xml, absPath and external links; see fixtures/README.md" >&2
    exit 1
fi
count=$(printf '%s\n' "$files" | grep -c .)
echo "fixtures: OK ($count workbooks)"
