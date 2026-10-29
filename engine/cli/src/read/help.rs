//! The long help of `read`: what it prints in each format. The JSON it
//! describes is a contract, so the help is where a script's author reads
//! it, and `docs/sdk.md` points here.

/// `read --help`'s description.
pub(crate) const LONG_ABOUT: &str = "\
Print a sheet's rows, one line each, as they are read: no more of the sheet
is held than the row being printed. Only the rows the sheet writes come, in
order, each with its number. Reading stops at the first problem in the
sheet; the rows before it have come.";

/// `read --help`'s text after the options.
pub(crate) const AFTER_LONG_HELP: &str = r##"Text (the default) is for a person and may change. Each row is a line: its
number, then the value of each column from A, separated by tabs. The
problems found, and why no sheet was read, go to stderr.

JSON (--format json) is JSON Lines, a contract: one object a line.

A row is {"row": 3, "cells": [CELL, ...]}: its number, from 1, and the
value of each column from A to its last cell, a column without one empty.
A CELL is {"type": TYPE, "value": VALUE}:

  TYPE      VALUE
  empty     null
  string    a string
  number    a number with a fraction or an exponent: 3.0, 0.1, 1e+16
  boolean   true or false
  date      "YYYY-MM-DD"
  time      "hh:mm:ss.sss"
  datetime  "YYYY-MM-DDThh:mm:ss.sss"
  elapsed   an integer: milliseconds, which do not wrap at a day
  error     the error value as written: "#N/A"

A date, a time or a datetime is the workbook's wall-clock time, with no
time zone: a number its cell's format shows as one, counted in the
workbook's date system and rounded to the millisecond.

The last line is the end:
{"ok": BOOL, "sheet": SHEET, "rows": N, "error": ERROR, "diagnostics": [...]}

  ok           no error, and no diagnostic of severity "error"
  sheet        {"index": N, "name": NAME}: the sheet asked for, 0 the
               first; null when there is none to choose
               (workbook_unreadable, sheet_not_found, sheet_ambiguous)
  rows         how many row lines came
  error        null, or why no sheet was read to its end:
               {"code": CODE, "message": TEXT}, CODE one of
               workbook_unreadable, sheet_not_found, sheet_ambiguous (two
               sheets have the name), sheet_not_worksheet (a chartsheet, a
               dialog sheet or a macro sheet) and sheet_unreadable
  diagnostics  every problem found in the workbook, as every host prints
               them

Exit status: 0 when ok, 1 when not. 1 too, with the reason on stderr and no
end line, when the host fails: the file cannot be opened or read, a
temporary file cannot be written, or the output cannot be written; the
last line may then be cut off. 2 for a command line not taken. When
stdout closes early (| head) while it is still writing, reading stops at
the next output it flushes, nothing more is said, and the status is 0."##;
