//! The long help of `sheets`: what it prints in each format. The JSON it
//! describes is a contract, so the help is where a script's author reads
//! it, and `docs/sdk.md` points here.

/// `sheets --help`'s description.
pub(crate) const LONG_ABOUT: &str = "\
List a workbook's sheets, in the order the workbook lists them: hidden ones
and every kind of sheet included, each with the index `read --sheet-index`
takes.";

/// `sheets --help`'s text after the options.
pub(crate) const AFTER_LONG_HELP: &str = r##"Text (the default) is for a person and may change. Each sheet is a line:
its index, name, kind and state, separated by tabs, the name's backslash,
tab and line ends written \\, \t, \n and \r. The problems found, and why
there is no list, go to stderr.

JSON (--format json) is one line, a contract:
{"ok": BOOL, "sheets": [SHEET, ...], "error": ERROR, "diagnostics": [...]}

  ok           no error, and no diagnostic of severity "error"
  sheets       each sheet, 0 the first:
               {"index": N, "name": NAME, "kind": KIND, "state": STATE}
               KIND is worksheet, chartsheet, dialogsheet, macrosheet, or
               unknown when which part the sheet is is not known (read
               refuses it as sheet_unreadable); STATE is visible, hidden
               or veryHidden. Empty when the workbook cannot be read.
  error        null, or why there is no list:
               {"code": "workbook_unreadable", "message": TEXT}
  diagnostics  every problem found in opening the workbook, as every host
               prints them; read prints them too

Exit status: 0 when ok, 1 when not. 1 too, with the reason on stderr and no
JSON, when the host fails: the file cannot be opened or read, a temporary
file cannot be written, or the output cannot be written. 2 for a command
line not taken. When stdout closes early, nothing more is said, and the
status is 0."##;
