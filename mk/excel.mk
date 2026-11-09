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
#          at spaces and a `*` in them is passed to the script as is; run
#          python3 -I scripts/excel-check directly for a path that contains a
#          space
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
	@set -f; set -- $$EXCEL_CHECK_F; \
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
# before it goes into fixtures/. Runs on the host with python3, not in
# Docker; it is not a gate. scripts/check-fixtures.sh, run by engine:budget,
# fails when a field it empties or removes still holds a value in a
# workbook in fixtures/.

.PHONY: fixtures\:scrub

fixtures\:scrub: export FIXTURES_SCRUB_F = $(value F)
fixtures\:scrub: ## Remove the account and organization names and the paths Excel saves from F='<xlsx>...', in place (before adding them to fixtures/)
	@set -f; set -- $$FIXTURES_SCRUB_F; \
	if [ $$# -eq 0 ]; then \
		echo "usage: make fixtures:scrub F='<xlsx>...'" >&2; \
		exit 2; \
	fi; \
	exec python3 -I scripts/fixtures-scrub "$$@"
