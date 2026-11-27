# excel — opening workbooks in Excel for Mac (scripts/excel-check)
#
# Not a gate: it runs on the host, not in Docker, only on macOS with Microsoft
# Excel installed, so it is not part of `make verify` and CI never runs it. It
# prints the script's JSON instead of a PASS/FAIL line. make exits 0 when the
# script exits 0 (every workbook opened and closed and every range was read)
# and 2 otherwise; make's "Error N" line gives the script's status, 1 when
# the JSON gives an error for a workbook and 2 when the script cannot run on
# this machine.
# scripts/excel-check says what the JSON holds and which macOS permissions it
# needs.
#
#   F      the .xlsx files to open, separated by spaces. The paths are split
#          at spaces and each path is expanded as a shell glob, such as
#          F='fixtures/generated/cells/*.xlsx'; a path that matches no file
#          is passed to the script as is, and scripts/excel-check reports
#          "no such file" for that path. Run python3 -I scripts/excel-check
#          directly for a path that contains a space, or a `*`, `?` or `[`
#          as a character rather than as a pattern
#   CELLS  one range whose displayed text is read from every workbook, such
#          as Sheet1!A1:C3 or 'a b'!A1; run the script directly to read more
#          than one range
#   OUT    a file to write the JSON to instead of standard output
#
# F, CELLS and OUT are passed to the recipe as the environment variables
# EXCEL_CHECK_F, EXCEL_CHECK_CELLS and EXCEL_CHECK_OUT, unexpanded by make
# ($(value ...)), so a quote or a `$` in them needs no escaping. F is defined
# (empty) in mk/engine.mk, which uses it for engine:test.

CELLS :=
OUT :=

.PHONY: excel\:check

excel\:check: export EXCEL_CHECK_F = $(value F)
excel\:check: export EXCEL_CHECK_CELLS = $(value CELLS)
excel\:check: export EXCEL_CHECK_OUT = $(value OUT)
excel\:check: ## Open F='<xlsx>...' in Excel for Mac and print the dialogs and CELLS='Sheet1!A1:C3' as JSON (OUT=<file>; macOS only, not in verify)
	@set -- $$EXCEL_CHECK_F; \
	if [ $$# -eq 0 ]; then \
		echo "usage: make excel:check F='<xlsx>...' [CELLS='Sheet1!A1:C3'] [OUT=<file>]" >&2; \
		exit 2; \
	fi; \
	set -- -- "$$@"; \
	if [ -n "$$EXCEL_CHECK_CELLS" ]; then set -- --cells="$$EXCEL_CHECK_CELLS" "$$@"; fi; \
	if [ -n "$$EXCEL_CHECK_OUT" ]; then set -- --out="$$EXCEL_CHECK_OUT" "$$@"; fi; \
	exec python3 -I scripts/excel-check "$$@"

# fixtures:scrub — removes from each workbook in F the account's name and the
# local paths Excel for Mac saves in it, in place (scripts/fixtures-scrub),
# before it goes into fixtures/. The paths in F are not expanded as a shell
# glob (set -f): scripts/fixtures-scrub rewrites each file in place, and a
# pattern could rewrite a file the caller did not name.
# Runs on the host with python3, not in Docker; it is not a gate.
# scripts/check-fixtures.sh, run by engine:budget, fails when a field it
# empties or removes still holds a value in a workbook in fixtures/.

.PHONY: fixtures\:scrub

fixtures\:scrub: export FIXTURES_SCRUB_F = $(value F)
fixtures\:scrub: ## Remove the account and organization names and the paths Excel saves from F='<xlsx>...', in place (before adding them to fixtures/)
	@set -f; set -- $$FIXTURES_SCRUB_F; \
	if [ $$# -eq 0 ]; then \
		echo "usage: make fixtures:scrub F='<xlsx>...'" >&2; \
		exit 2; \
	fi; \
	exec python3 -I scripts/fixtures-scrub "$$@"

# fixtures:generate — writes into fixtures/generated/, which git ignores
# (.gitignore), workbooks too large to commit, for the reader's benchmarks
# (`make bench:compare`, `make bench:cli`, bench/ruby/) and, later, tests on
# large workbooks: rows-100000.xlsx, rows-1000000.xlsx, shared-100000.xlsx and
# shared-1000000.xlsx, a header row and that many data rows each (a rows
# book has its strings in its cells; a shared book has strings all
# different, in the shared strings table), written by rust_xlsxwriter
# (engine/fixtures-gen, whose write_rows_book and write_shared_book say
# what a workbook holds). Runs in the toolchain image like the engine's
# gates, but is not a gate: it prints the release build's output and the
# path of each workbook written, and proves nothing. A workbook already
# there under the same name is replaced.
# scripts/check-fixtures.sh skips fixtures/generated/ and fails when git
# tracks a file in it.

.PHONY: fixtures\:generate

fixtures\:generate: ## Write the large workbooks (rows and shared strings, 100,000 and 1,000,000 rows each) into fixtures/generated/ (gitignored) with engine/fixtures-gen
	$(CARGO_IN_DOCKER) 'cd /repo && cargo run --manifest-path engine/Cargo.toml -p xlsx-graft-fixtures-gen \
		--release --locked $(CARGO_JOBS) -- fixtures/generated'

# fixtures:repack — writes each workbook of fixtures/ again with the engine's
# writer (engine/write's PackageEdit) into fixtures/generated/repacked/, which
# git ignores, for `make excel:check` to open on a Mac and show that Excel
# accepts what the writer makes: <name>, its package edited by nothing
# (every entry copied as it is, the zip laid out again), and <stem>-app.<ext>,
# with the text of the Application element of docProps/app.xml replaced (one
# part rewritten, the others copied). A workbook with no docProps/app.xml
# gives the first only. Runs in the toolchain image like fixtures:generate,
# and is not a gate and not a member of verify: it prints the release
# build's output and the path of each workbook written, and proves nothing.
# A workbook already there under the same name is replaced.

.PHONY: fixtures\:repack

fixtures\:repack: ## Write each workbook of fixtures/ again with the engine's writer (as it is, and with the Application in docProps/app.xml rewritten) into fixtures/generated/repacked/ (gitignored), for make excel:check
	$(CARGO_IN_DOCKER) 'cd /repo && cargo run --manifest-path engine/Cargo.toml -p xlsx-graft-fixtures-gen \
		--release --locked $(CARGO_JOBS) -- repack fixtures fixtures/generated/repacked'

# fixtures:cells — writes three workbooks of cell values with the engine's
# cell writer (engine/cell's write_cell and SharedStringsWriter, in zips from
# engine/write) into fixtures/generated/cells/, which git ignores:
# cells-1900.xlsx (the 1900 date system that counts 1900-02-29),
# cells-1900-strict.xlsx (conformance="strict", the standard's 1900) and
# cells-1904.xlsx (date1904="1"). Each has a row for each kind of value, a
# description in column A and the value in column B: strings that need _xHHHH_
# escapes or are cut at 32,767 UTF-16 code units, numbers at their limits,
# booleans, error values, dates, times, date-times and elapsed times, and
# shared strings; a value the cell writer refuses is the text "refused: <why>".
# On a Mac, make excel:check F='fixtures/generated/cells/*.xlsx' tells
# whether Excel opens them without a repair. Runs in the toolchain image like
# fixtures:repack, and is not a gate and not a member of verify: it prints the
# release build's output and the path of each workbook written, and proves
# nothing. A workbook already there under the same name is replaced.

.PHONY: fixtures\:cells

fixtures\:cells: ## Write three workbooks of cell values with the engine's cell writer into fixtures/generated/cells/ (gitignored), for make excel:check
	$(CARGO_IN_DOCKER) 'cd /repo && cargo run --manifest-path engine/Cargo.toml -p xlsx-graft-fixtures-gen \
		--release --locked $(CARGO_JOBS) -- cells fixtures/generated/cells'

# fixtures:styles — writes each workbook of fixtures/ again with a cell format
# of the engine's style crate (engine/style's StylesEdit) on each of the cells
# A1 to A18 of its first worksheet, into fixtures/generated/styles/, which git
# ignores, for `make excel:check` to open on a Mac and show that Excel accepts
# the styles part StylesEdit writes and shows each format as it is meant to be
# shown: bold, italic, underline, colours, fonts, fills, borders, number
# formats (dates, times, elapsed time, thousands, percent), alignment and
# protection, in the order engine/fixtures-gen's src/styles/catalog.rs lists.
# A cell column A already has keeps its value and gets the format; a row
# without one gets a cell with a value that shows the format (the number
# 45292.75 for a number format, a string that names the format otherwise),
# and has its spans attribute removed; a row the sheet lacks is added. The
# dimension is not changed, so a cell added outside it is a thing to look for
# when Excel repairs a workbook. A workbook with no worksheet or no styles part gives
# nothing (a workbook whose sheets are all chartsheets, say); one whose
# package cannot be read, or whose workbook.xml is not XML or is not allowed
# by the schema (a <sheets> with no <sheet>, say), or which has two styles
# relationships to different parts, is refused. Runs in the
# toolchain image like fixtures:generate, and is not a gate and not a member
# of verify: it prints the release build's output and the path of each
# workbook written, and proves nothing. A workbook already there under the
# same name is replaced.

.PHONY: fixtures\:styles

fixtures\:styles: ## Write each workbook of fixtures/ again with a cell format on each of A1 to A18 of its first worksheet into fixtures/generated/styles/ (gitignored), for make excel:check
	$(CARGO_IN_DOCKER) 'cd /repo && cargo run --manifest-path engine/Cargo.toml -p xlsx-graft-fixtures-gen \
		--release --locked $(CARGO_JOBS) -- styles fixtures fixtures/generated/styles'

# fixtures:structure — writes each workbook of fixtures/ twice with the
# engine's structure edit (engine/write's StructureEdit) into
# fixtures/generated/structure/, which git ignores, for `make excel:check` to
# open on a Mac and show that Excel accepts the sheets and the formulas the
# edit writes: <stem>-rename.<ext>, its first sheet renamed to `Graft 名前 1`,
# a sheet `Graft added` added at position 0, its last sheet moved to position
# 1 and the workbook's defined name graft_name set to 'Graft 名前 1'!$A$1;
# and <stem>-delete.<ext>, a sheet `Graft added` added after the last sheet
# and the first sheet deleted. The formulas of the other parts (the sheets',
# the charts', the tables', the pivot caches') follow the rename and the
# delete as the edit rewrites them. The extension is the workbook's own, so
# macro.xlsm and python-in-excel.xlsm give .xlsm files. Standard output has
# the path of each workbook written and a line for each reference the edit
# kept as it was (a StructureReport); a workbook, or one of its two variants,
# the edit refuses or cannot read is skipped, with `skipped`, its path and why
# on a line, and the others are written. Runs in the toolchain image like
# fixtures:styles, and is not a gate and not a member of verify: it prints the
# release build's output, and proves nothing. A workbook already there under
# the same name is replaced.

.PHONY: fixtures\:structure

fixtures\:structure: ## Write each workbook of fixtures/ with a sheet renamed, added and moved, and with the first sheet deleted, into fixtures/generated/structure/ (gitignored), for make excel:check
	$(CARGO_IN_DOCKER) 'cd /repo && cargo run --manifest-path engine/Cargo.toml -p xlsx-graft-fixtures-gen \
		--release --locked $(CARGO_JOBS) -- structure fixtures fixtures/generated/structure'

# fixtures:layout — writes each workbook of fixtures/ again with the layout of
# its first sheet and the visibility of its sheets edited by the engine
# (engine/write's SheetEdit and StructureEdit) into
# fixtures/generated/layout/, which git ignores, for `make excel:check` to
# open on a Mac and show that Excel accepts what the edits write. On the first
# sheet, when it is a worksheet: cells AA1000:AB1001 merged (AD2000:AE2001 for
# a sheet that has a range or a table over a cell of the first), columns B to
# D 20 wide and F 30 wide (a `col` of the sheet that covers them is cut), row
# 1 30 points high and row 500 24 points high (a row the sheet lacks is
# written), the first row and the first column frozen, and the sheet
# protected with the password `xlsx-graft` (SHA-512, the salt bytes 0 to 15,
# spin count 100000, the default permissions). In the workbook: with two
# sheets or more the last sheet is the active one, with three or more the
# second is hidden, with four or more the third is very hidden. The name and
# the extension are the workbook's own, so macro.xlsm and python-in-excel.xlsm
# give .xlsm files. Standard output has the path of each workbook written and
# an indented line for each edit the engine refused and so was not made (or a
# merge made at the second range); a workbook the engine refuses or cannot
# read is skipped, with `skipped`, its path and why on a line, and the others
# are written. Runs in the toolchain image like fixtures:structure, and is not
# a gate and not a member of verify: it prints the release build's output, and
# proves nothing. A workbook already there under the same name is replaced.

.PHONY: fixtures\:layout

fixtures\:layout: ## Write each workbook of fixtures/ with its first sheet's cells merged, columns and rows sized, panes frozen and sheet protected, and its sheets made active, hidden and very hidden, into fixtures/generated/layout/ (gitignored), for make excel:check
	$(CARGO_IN_DOCKER) 'cd /repo && cargo run --manifest-path engine/Cargo.toml -p xlsx-graft-fixtures-gen \
		--release --locked $(CARGO_JOBS) -- layout fixtures fixtures/generated/layout'
