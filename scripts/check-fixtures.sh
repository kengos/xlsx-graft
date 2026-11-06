#!/bin/sh
# The workbooks in fixtures/ match fixtures/README.md and carry no names or
# local paths.
#
# fixtures/README.md lists each workbook in one row of its table, the row's
# first cell the file name in backticks:
#
#     | `table.xlsx` | ... |
#
# Every .xlsx and .xlsm file directly in fixtures/ has such a row, and every
# such row names a file there. Each of those workbooks, as fixtures-scrub
# leaves it:
#
#   - docProps/core.xml has no text in dc:creator or cp:lastModifiedBy,
#     which Excel fills with the name of the account that saved it;
#   - no entry holds `absPath` (x15ac:absPath, the folder Excel saved it
#     in) or a macOS or Windows home folder (`/Users/`, `:\Users\`), which
#     Excel also writes as the absolute path of a linked workbook.
#
# scripts/fixtures-scrub removes all of them; run `make fixtures:scrub`.
#
# Read-only POSIX sh + standard tools (unzip); no toolchain required.
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
    echo "fixtures: no workbook found in $dir — the scan is broken, not clean" >&2
    exit 1
fi

violations=$(
    printf '%s\n' "$files" | while IFS= read -r f; do
        printf '%s\n' "$listed" | grep -qxF "$f" ||
            echo "$f has no row in fixtures/README.md"
    done
    printf '%s\n' "$listed" | grep . | uniq -d | while IFS= read -r f; do
        echo "$f has more than one row in fixtures/README.md"
    done
    printf '%s\n' "$listed" | grep . | while IFS= read -r f; do
        printf '%s\n' "$files" | grep -qxF "$f" ||
            echo "fixtures/README.md lists $f, which is not in fixtures/"
    done
    printf '%s\n' "$files" | while IFS= read -r f; do
        book="$dir/$f"
        if ! unzip -tqq "$book" >/dev/null 2>&1; then
            echo "$f is not a zip unzip can read"
            continue
        fi
        core=$(unzip -p "$book" docProps/core.xml 2>/dev/null || true)
        if printf '%s' "$core" | grep -q '<dc:creator>[^<]'; then
            echo "$f: docProps/core.xml has a dc:creator"
        fi
        if printf '%s' "$core" | grep -q '<cp:lastModifiedBy>[^<]'; then
            echo "$f: docProps/core.xml has a cp:lastModifiedBy"
        fi
        if unzip -p "$book" | LC_ALL=C grep -aq 'absPath'; then
            echo "$f holds an absPath"
        fi
        if unzip -p "$book" | LC_ALL=C grep -aqF -e '/Users/' -e ':\Users\'; then
            echo "$f holds a home folder path (/Users/ or :\\Users\\)"
        fi
    done
)

if [ -n "$violations" ]; then
    printf '%s\n' "$violations" | sed 's/^/fixtures: /' >&2
    echo "fixtures: python3 -I scripts/fixtures-scrub removes names and paths; see fixtures/README.md" >&2
    exit 1
fi
count=$(printf '%s\n' "$files" | grep -c .)
echo "fixtures: OK ($count workbooks)"
