#!/bin/sh
# The workbooks in fixtures/ match the rows of fixtures/README.md, hold only
# the kinds of part listed below, and have the fields where Excel saves an
# account's name or a local path in the form scripts/fixtures-scrub leaves.
#
# fixtures/README.md lists each workbook in one row of its table, the row's
# first cell the file name in backticks:
#
#     | `table.xlsx` | ... |
#
# fixtures/ holds README.md, the AppleScripts that made the workbooks
# (fixtures/scripts/*.applescript) and the workbooks, each a .xlsx or .xlsm
# file directly in fixtures/; any other file there, an untracked Finder
# .DS_Store directly in fixtures/ or fixtures/scripts/ aside, fails the
# check, and so does a symbolic link, so a workbook cannot pass unchecked
# under another extension, in a subfolder or outside fixtures/; the
# gitignored fixtures/generated/ is the one subfolder skipped (below). Every
# workbook has a row, and every row names a workbook there.
#
# fixtures/generated/ holds the workbooks `make fixtures:generate` writes,
# which git ignores (.gitignore). The check reads nothing in it, and fails
# when git tracks a file in it (or, without git or outside a git work
# tree, when it exists), so a workbook there is never committed unchecked.
#
# The check covers what Excel writes when it saves a workbook: the parts
# and fields below, in the forms Excel writes them (UTF-8 XML, any
# namespace prefix). A part edited by hand, as fixtures/README.md describes
# for python-in-excel.xlsm, is read by the person who edits it; the check
# does not look for a name written in a form Excel does not write, such as
# a UTF-16 part. A VBA project is the exception: Excel writes paths into it
# in UTF-16 and in a compressed stream, and the check compares the whole
# part with the one a person read instead of searching it (below).
#
# Each workbook:
#
#   - holds only the parts `allowed_part` lists, matched by name in any
#     letter case (OPC compares part names that way), each pattern within
#     the folder it names. A part of any other kind fails the check
#     without being read: the parts Excel writes an account into
#     (docProps/custom.xml, comments, persons, threaded comments,
#     docMetadata/LabelInfo.xml, a web image's rich value and others) are
#     not listed;
#   - has, in docProps/core.xml, each creator and lastModifiedBy element
#     empty, and in docProps/app.xml each Company, Manager and
#     HyperlinkBase element empty, written as `<x></x>` or `<x/>` with any
#     prefix, so a value in CDATA or after a line break fails the check as
#     well;
#   - holds no `absPath` (the folder Excel saved the workbook in), no
#     `alternateUrls` (the absolute path of a linked workbook), no
#     `refreshedBy` (the account that last refreshed a pivot cache), no
#     `fileSharing` (which names the account that protected the workbook)
#     and no `securityDescriptor` (the accounts allowed to edit a
#     protected range) in any part;
#   - has, as the Target of each relationship with TargetMode="External",
#     a file name alone (no `/`, `\` or `:`), which a linked workbook in
#     the same folder has; a URL or a path names a folder, a tenant or an
#     account;
#   - holds the Office Script list customXml/item*.xml and its properties
#     customXml/itemProps*.xml, if it has them, as `$office_script_ids`
#     and `$office_script_props` give them (carriage returns and trailing
#     line feeds removed; the other line feeds match where they are): the
#     placeholder IDs fixtures/README.md describes in place of a OneDrive
#     item ID and a sharing link that name a tenant and an account;
#   - holds a VBA project xl/vbaProject.bin only if the workbook is
#     macro.xlsm and the part's SHA-256 is `$vba_project_sha256`. Excel for
#     Mac writes into every VBA project a reference to Microsoft Forms 2.0
#     whose path is in the account's home folder
#     (/Users/<account>/Library/Containers/com.microsoft.Excel/Data/Library/Preferences/MSForms.exd),
#     and the VBA editor does not let the reference be removed. The home
#     folder search below, which matches ASCII text, cannot find the path
#     reliably: one copy of it is in UTF-16, and the compression splits
#     the other copy into pieces, though it can leave `/Users/` whole. So
#     the check compares the whole part with the one a person has read,
#     which fixtures/README.md describes, and once the SHA-256 matches, the
#     home folder search skips that part.
#
# A search for a macOS or Windows home folder (`/Users/`, `:\Users\`, or
# `:Users:` as Excel for Mac 2011 wrote it, in any letter case) runs over
# every part, README.md and the AppleScripts as well, and finds a path in
# a field the checks above do not name.
#
# The check does not look at every place a workbook can hold a name, and
# is not meant to: the person who adds a workbook reads everything
# fixtures/README.md lists under "What the check does not read" (cell and
# shape text, headers and footers, document properties, the values of rich
# data and linked data types, and more).
#
# scripts/fixtures-scrub writes the core.xml and app.xml fields empty and
# removes absPath, a linked workbook's absolute path and a pivot cache's
# refreshedBy; run `make fixtures:scrub`. A part outside the list is
# removed by hand, or added to `allowed_part` once a person has read it
# and found no account in it. A fileSharing element, a
# securityDescriptor, an external target that is not a file name and an
# Office Script list are removed or replaced by hand. fixtures/README.md
# says the same.
#
# POSIX sh with the find, grep and unzip of macOS and Linux, sha256sum
# (Linux) or shasum (macOS), and git to tell whether a .DS_Store is tracked
# (without git, or outside a git work tree, a .DS_Store fails the check); no
# toolchain required.
set -eu

root=$(dirname "$0")/..
dir="$root/fixtures"
readme="$dir/README.md"

if ! command -v unzip >/dev/null 2>&1; then
    echo "fixtures: unzip is not installed, and this check reads each workbook with it" >&2
    exit 1
fi
if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
    echo "fixtures: neither sha256sum nor shasum is installed, and this check compares macro.xlsm's VBA project by its SHA-256 with one of them" >&2
    exit 1
fi
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

# The Office Script list of python-in-excel.xlsm, with the placeholder IDs
# fixtures/README.md describes, and its properties part with its line
# breaks as `\n`.
office_script_ids='<scriptIds xmlns="http://schemas.microsoft.com/office/extensibility/maker/v1.0" id="script-ids-node-id"><scriptId id="ms-officescript%3A%2F%2Fonedrive_business_itemlink%2F01AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA:ms-officescript%3A%2F%2Fonedrive_business_sharinglink%2Fu!aHR0cHM6Ly9leGFtcGxlLW15LnNoYXJlcG9pbnQuY29tLzp1Oi9nL3BlcnNvbmFsL3VzZXJfZXhhbXBsZV9vbm1pY3Jvc29mdF9jb20vQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFB"/></scriptIds>'
office_script_props='<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<ds:datastoreItem ds:itemID="{37CC2A93-D507-FE44-9B4E-272F583B10EE}" xmlns:ds="http://schemas.openxmlformats.org/officeDocument/2006/customXml"><ds:schemaRefs><ds:schemaRef ds:uri="http://schemas.microsoft.com/office/extensibility/maker/v1.0"/></ds:schemaRefs></ds:datastoreItem>'

# The SHA-256 of macro.xlsm's xl/vbaProject.bin, read as fixtures/README.md
# describes: the module Module1 with Sub AddOne, and the references Excel
# for Mac writes, one of them the MSForms.exd path in the home folder.
vba_project_sha256=55c2c054a601b939b10761f54683cd87b739fbee4d84c5741b8a92a6870b4034

# The SHA-256 of standard input, in lowercase hexadecimal.
sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | cut -c1-64
    else
        shasum -a 256 | cut -c1-64
    fi
}

# Whether the part name $1, in lowercase, is of a kind a workbook here may
# hold: the parts of a workbook, its sheets, styles and theme, and the
# drawings, charts, tables, external links, pivot tables, rich data,
# Python code, Office Script list and VBA project the workbooks here test.
allowed_part() {
    # Each pattern is an extended regular expression matched against the
    # whole name (`grep -x`). Each names its folder literally and matches
    # only a number or a fixed file name within it, so no pattern matches
    # a part in another folder.
    printf '%s\n' "$1" | LC_ALL=C grep -qxE \
        -e '\[content_types\]\.xml|_rels/\.rels|docprops/(core|app)\.xml' \
        -e 'xl/(workbook|styles|sharedstrings|calcchain|metadata|python)\.xml' \
        -e 'xl/vbaproject\.bin' \
        -e 'xl/_rels/workbook\.xml\.rels' \
        -e 'xl/theme/theme[0-9]+\.xml|xl/media/image[0-9]+\.png' \
        -e 'xl/(worksheets/sheet|drawings/drawing|charts/chart|tables/table)[0-9]+\.xml' \
        -e 'xl/(worksheets/_rels/sheet|drawings/_rels/drawing|charts/_rels/chart)[0-9]+\.xml\.rels' \
        -e 'xl/charts/(style|colors)[0-9]+\.xml' \
        -e 'xl/externallinks/externallink[0-9]+\.xml|xl/externallinks/_rels/externallink[0-9]+\.xml\.rels' \
        -e 'xl/pivotcache/pivotcache(definition|records)[0-9]+\.xml' \
        -e 'xl/pivotcache/_rels/pivotcachedefinition[0-9]+\.xml\.rels' \
        -e 'xl/pivottables/pivottable[0-9]+\.xml|xl/pivottables/_rels/pivottable[0-9]+\.xml\.rels' \
        -e 'xl/richdata/(rdrichvalue|rdrichvaluestructure|rdrichvaluetypes|rdarray|richstyles|richvaluerel)\.xml' \
        -e 'xl/richdata/rdsupportingpropertybag(structure)?\.xml|xl/richdata/_rels/richvaluerel\.xml\.rels' \
        -e 'customxml/item(props)?[0-9]+\.xml|customxml/_rels/item[0-9]+\.xml\.rels'
}

# The entries of the zip $1 whose names match $2 (an unzip wildcard, `*`
# for every entry, in any letter case), with each line break and tab
# replaced by a space. An entry pattern no entry matches gives nothing.
entries() {
    { unzip -C -p "$1" "$2" 2>/dev/null || true; } | LC_ALL=C tr '\n\r\t' '   '
}

# Whether the text on standard input has each element of the local name
# $1, with any namespace prefix, empty: as many start tags of it as empty
# elements (`<x></x>` or `<x/>`, with or without attributes).
all_empty() {
    text=$(cat)
    tag="([A-Za-z_][^ :>/]*:)?$1"
    starts=$(printf '%s' "$text" | LC_ALL=C grep -aoE "<$tag[ />]" | grep -c . || true)
    empty=$(printf '%s' "$text" |
        LC_ALL=C grep -aoE "<$tag( [^>/]*)?(/>|></$tag>)" | grep -c . || true)
    [ "$starts" = "$empty" ]
}

violations=$(
    # git ls-files exits 0 with the tracked files under fixtures/generated/
    # (none, when git tracks none), and another status without git or
    # outside a git work tree.
    tracked=$(git -C "$root" ls-files -- fixtures/generated 2>/dev/null) || tracked=-
    if [ "$tracked" = - ]; then
        if [ -e "$dir/generated" ] || [ -L "$dir/generated" ]; then
            printf '%s\n' "generated/ cannot be told untracked without git or outside a git work tree; remove it"
        fi
    elif [ -n "$tracked" ]; then
        printf '%s\n' "$tracked" | while IFS= read -r f; do
            printf '%s\n' "${f#fixtures/} is tracked by git; fixtures/generated/ holds the workbooks make fixtures:generate writes, which are not committed"
        done
    fi
    find "$dir" -path "$dir/generated" -prune -o ! -type d -print | sed "s|^$dir/||" | LC_ALL=C sort | while IFS= read -r f; do
        if [ -L "$dir/$f" ]; then
            printf '%s\n' "$f is a symbolic link; fixtures/ holds files only"
            continue
        fi
        case $f in
        (.DS_Store | scripts/.DS_Store) # Finder's, untracked (.gitignore)
            # git ls-files exits 0 for a tracked file, 1 for an untracked
            # one, and another status outside a git work tree.
            status=0
            if command -v git >/dev/null 2>&1; then
                git -C "$root" ls-files --error-unmatch -- "fixtures/$f" >/dev/null 2>&1 || status=$?
            else
                status=127
            fi
            case $status in
            (0) printf '%s\n' "$f is tracked by git; Finder's .DS_Store, which lists file names, stays untracked" ;;
            (1) ;;
            (*) printf '%s\n' "$f cannot be told untracked without git or outside a git work tree; remove it" ;;
            esac
            ;;
        (scripts/*/*) printf '%s\n' "$f is in a subfolder of scripts/; the AppleScripts are directly in fixtures/scripts/" ;;
        (README.md | scripts/*.applescript)
            if LC_ALL=C grep -qiF -e '/Users/' -e ':\Users\' -e ':Users:' "$dir/$f"; then
                printf '%s\n' "$f holds a home folder path (/Users/, :\\Users\\ or :Users:)"
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
        unzip -Z1 "$book" | while IFS= read -r part; do
            name=$(printf '%s' "$part" | LC_ALL=C tr 'A-Z' 'a-z')
            if ! allowed_part "$name"; then
                printf '%s\n' "$f: $part is not a kind of part scripts/check-fixtures.sh allows"
            fi
            case $name in
            (customxml/itemprops[0-9]*.xml)
                if [ "$(unzip -p "$book" "$part" | tr -d '\r')" != "$office_script_props" ]; then
                    printf '%s\n' "$f: $part is not the properties of the Office Script list"
                fi
                ;;
            (customxml/item[0-9]*.xml)
                if [ "$(unzip -p "$book" "$part" | tr -d '\r')" != "$office_script_ids" ]; then
                    printf '%s\n' "$f: $part is not the Office Script list with the placeholder IDs"
                fi
                ;;
            (xl/vbaproject.bin)
                if [ "$f" != macro.xlsm ] ||
                    [ "$(unzip -p "$book" "$part" | sha256)" != "$vba_project_sha256" ]; then
                    printf '%s\n' "$f: $part is not macro.xlsm's VBA project as fixtures/README.md describes it, the one part allowed to hold a home folder path"
                fi
                ;;
            (*.rels)
                if unzip -p "$book" "$part" | LC_ALL=C tr '\n\r\t' '   ' |
                    LC_ALL=C grep -aoE '<Relationship [^>]*TargetMode="External"[^>]*>' |
                    LC_ALL=C grep -aoE 'Target="[^"]*"' |
                    LC_ALL=C grep -avxE 'Target="[^/\\:"]+"' >/dev/null; then
                    printf '%s\n' "$f: $part has an external Target that is not a file name alone"
                fi
                ;;
            esac
        done
        for field in creator lastModifiedBy; do
            if ! entries "$book" docProps/core.xml | all_empty "$field"; then
                printf '%s\n' "$f: docProps/core.xml has a $field that is not empty"
            fi
        done
        for field in Company Manager HyperlinkBase; do
            if ! entries "$book" docProps/app.xml | all_empty "$field"; then
                printf '%s\n' "$f: docProps/app.xml has a $field that is not empty"
            fi
        done
        for text in absPath alternateUrls refreshedBy fileSharing securityDescriptor; do
            if entries "$book" '*' | LC_ALL=C grep -aqF "$text"; then
                printf '%s\n' "$f holds $text"
            fi
        done
        # macro.xlsm's VBA project, once its SHA-256 matches, holds the
        # MSForms.exd path fixtures/README.md describes.
        skip=
        if [ "$f" = macro.xlsm ] &&
            [ "$(unzip -C -p "$book" xl/vbaProject.bin 2>/dev/null | sha256)" = "$vba_project_sha256" ]; then
            skip=xl/vbaProject.bin
        fi
        if { unzip -C -p "$book" '*' ${skip:+-x "$skip"} 2>/dev/null || true; } |
            LC_ALL=C grep -aqiF -e '/Users/' -e ':\Users\' -e ':Users:'; then
            printf '%s\n' "$f holds a home folder path (/Users/, :\\Users\\ or :Users:)"
        fi
    done
)

if [ -n "$violations" ]; then
    printf '%s\n' "$violations" | sed 's/^/fixtures: /' >&2
    echo "fixtures: make fixtures:scrub F=<workbook> empties or removes the names and paths in core.xml, app.xml, absPath, external links and pivot caches; for the rest, see fixtures/README.md" >&2
    exit 1
fi
count=$(printf '%s\n' "$files" | grep -c .)
echo "fixtures: OK ($count workbooks)"
