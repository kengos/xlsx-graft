//! The long help of `describe-sheet`: what it prints in each format, what
//! bounds it and how it exits. The JSON it describes is a contract, so the
//! help is where a script's author reads it, and `docs/sdk.md` points here.

/// `describe-sheet --help`'s description.
pub(crate) const LONG_ABOUT: &str = "\
Show the cells of one sheet: the first rows and columns of its cells that hold
a value, or the part you ask for with --range, as a table. It is for checking
what a sheet holds; read prints every row. The C ABI and the MCP server give
the same view.";

/// `describe-sheet --help`'s text after the options.
pub(crate) const AFTER_LONG_HELP: &str = r##"Text (the default) is for a person and may change; it is not a contract. It is
a line naming the sheet and its ranges, a Markdown table of the cells shown
with the column letters above and the row numbers on the left, Merged: and the
merged ranges that overlap the range shown, a line each for rows, columns and
merged ranges left out, the diagnostics, then a line of the number of cells
listed, the counts of the diagnostics and whether the view is ok. When shown
is null the table is replaced by "nothing to show", and when the range shown
has more than 100,000 cells it is replaced by a list of the cells with a
value, one a line. All of it goes to stdout.

JSON (--format json) is one line, a contract:
{"ok": BOOL, "error": ERROR, "diagnostics": [...], "sheet": SHEET or null,
 "dimension": TEXT or null, "used": RANGE or null, "shown": RANGE or null,
 "truncated": {"rows": BOOL, "columns": BOOL, "merges": BOOL},
 "complete": BOOL, "rows": [{"row": N, "cells": [CELL, ...]}],
 "merges": [RANGE, ...]}

  ok           error is null, and no diagnostic is an error
  error        null, or {"code": CODE, "message": TEXT}, CODE one of io (the
               file cannot be opened, is not a regular file, stopped being
               readable while it was read, or the temporary file for a large
               shared strings table cannot be written), workbook_unreadable,
               sheet_not_found, sheet_ambiguous (two sheets have the name),
               sheet_not_worksheet (a chartsheet, a dialog sheet or a macro
               sheet) and sheet_unreadable
  diagnostics  every problem found, as every host prints them
  sheet        {"index": N, "name": TEXT, "kind": KIND, "state": STATE} as
               sheets prints a sheet; null when no sheet was chosen, or when
               the file could not be opened or read before one was chosen
               (io). sheet is set when the io error occurred while the sheet's
               rows were read
  dimension    the sheet's dimension as written, which writers do not all
               keep right; null when it has none
  used         the range of the cells that hold a value, in the whole sheet,
               whatever --range says; null when none does
  shown        the range looked at: from the top left cell of --range (A1
               without it), at most --max-rows rows and --max-columns columns,
               never past --range and never past the last row and column of
               used; null when shown would hold no cell
  truncated    rows: cells with a value lie below shown; columns: some lie to
               the right of shown; merges: merged ranges were left out
  complete     whether the sheet was read to its end with nothing stopping it
  rows         the rows of shown that have a cell with a value, from the top
  merges       the merged ranges that overlap shown, in the sheet's order

Every range (used, shown, each of merges) is written with two corners: A1:A1
for one cell; dimension is as the sheet wrote it. A CELL is
{"ref": "B2", "type": TYPE, "value": VALUE}, with TYPE and VALUE as read
gives them (see read --help), except that a cell with no value is not listed,
so empty does not occur. A CELL has a fourth key, "truncated": true, only when
the text of a string or an error value was cut; without it the text is whole,
and its value is always true.

Bounds, whatever the options: at most 100,000 cells with a value are listed,
each string cut at 1,000 characters and 1,000,000 characters in all, at most
100,000 merged ranges. A row that would pass the limit of cells or the limit
of characters, and the rows below it, are left out whole and set
truncated.rows. --max-rows is 1 to 10000 (100 without it) and --max-columns
1 to 1000 (30 without it). A sheet of long strings is cut short even in the
default window. A result is some 15 MB of JSON or 13 MB of text at the most,
without the diagnostics.

A quoted value (the sheet's name, dimension, and the text of a string or an
error value) is quoted from the workbook and tidied for display: control
characters (other than the tab, the line feed and the carriage return, which
a cell's text keeps), bidirectional formatting characters, the line and
paragraph separators (U+2028, U+2029) and the tag characters (U+E0000 to
U+E007F) are removed. A cell's text over 1,000 characters is cut, and
truncated says so; a name or a dimension over 200 characters is cut and ends
in "…". The exact text of a cell comes from read and the exact name of the
sheet from sheets; no command prints the dimension exactly.

Like verify, describe-sheet prints its view whatever happened to the workbook:
the object is on stdout whether the status is 0 or 1, and once it is printed
stderr stays empty.

Exit status: 0 when ok. 1 when not ok, including when the file cannot be
opened or the sheet cannot be chosen or read (error says why). 2 for a
command line not taken: clap's message, or the reason for a --range that is
no A1 range of cells or a --max-rows or --max-columns out of its bounds, on
stderr, and no object. When stdout closes early, nothing more is said, and
the status is 0.

Examples, from the repository's root:
  xlsx-graft describe-sheet fixtures/merged.xlsx
  xlsx-graft describe-sheet fixtures/merged.xlsx --range A1:C3 --format json"##;
