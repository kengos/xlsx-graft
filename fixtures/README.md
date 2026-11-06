# fixtures

Workbooks saved by Excel, for tests and manual checks that need a workbook
Excel wrote rather than one the tests build. Each has one row below: what it
is for, the Excel that saved it, how it was made, and what
`make excel:check` saw when Excel opened it.

What reading and verifying each one gives is fixed by the engine's tests in
[`engine/verify/tests/fixtures/expected.rs`](../engine/verify/tests/fixtures/expected.rs),
which read every workbook here.

| File | What it is for | Saved by | How it was made | `make excel:check` |
| --- | --- | --- | --- | --- |
| `chart.xlsx` | A clustered column chart (chart and drawing parts) | Excel for Mac 16.114.4 | [`scripts/chart.applescript`](scripts/chart.applescript) | No repair, no dialog (2026-11-06) |
| `date1904.xlsx` | The 1904 date system (`workbookPr@date1904`), dates and a date-time under date formats | Excel for Mac 16.114.4 | [`scripts/date1904.applescript`](scripts/date1904.applescript) | No repair, no dialog (2026-11-06) |
| `dynamic-array.xlsx` | Formulas that spill (`SEQUENCE`, `SORT`, `FILTER`, `UNIQUE`) and a spill reference (`A1#`) | Excel for Mac 16.114.4 | [`scripts/dynamic-array.applescript`](scripts/dynamic-array.applescript) | No repair, no dialog (2026-11-06) |
| `external-link.xlsx` | Formulas that refer to another workbook, `external-source.xlsx` (an external workbook reference part) | Excel for Mac 16.114.4 | [`scripts/external-link.applescript`](scripts/external-link.applescript) | No repair; Excel asks whether to update the link, answered "Don't Update" (2026-11-06) |
| `external-source.xlsx` | The workbook `external-link.xlsx` refers to | Excel for Mac 16.114.4 | [`scripts/external-link.applescript`](scripts/external-link.applescript) | No repair, no dialog (2026-11-06) |
| `formulas.xlsx` | Formulas the formula lexer was checked against: defined names, `LAMBDA`, a table's structured references, quoted sheet names, `WEBSERVICE` | Excel for Mac 16 (the release was not recorded) | [Below](#formulasxlsx) | No repair, no dialog (2026-11-06) |
| `table.xlsx` | A table with a calculated column and a totals row, and a formula outside it that refers to a column of it | Excel for Mac 16.114.4 | [`scripts/table.applescript`](scripts/table.applescript) | No repair, no dialog (2026-11-06) |

Every workbook went through `make fixtures:scrub` before it was added
([Adding a workbook](#adding-a-workbook)). "No repair" means
`make excel:check` gave `repair_dialog: false` and
`error: null`; "no dialog" that it also gave `dialogs: []`.

## Adding a workbook

1. Save it in Excel, with neutral sheet names and values (`Sheet1`, `Data`,
   numbers and English words).
2. Copy it here and run `make fixtures:scrub F=fixtures/<name>.xlsx`. Excel
   writes the account's name into `docProps/core.xml` (`dc:creator`,
   `cp:lastModifiedBy`), the folder it saved the workbook in
   (`x15ac:absPath`) into the workbook part, and the absolute path of a
   linked workbook into an external link (`xxl21:alternateUrls`); the
   script removes all of them and leaves the rest as Excel wrote it.
3. Run `make excel:check F=fixtures/<name>.xlsx` on a Mac with Excel and
   note what it saw.
4. Add the workbook's row above, and its entry to
   `engine/verify/tests/fixtures/expected.rs`.

`make engine:budget` (part of `make engine:verify`) runs
`scripts/check-fixtures.sh`, which fails when a workbook here has no row
above, a row names a workbook that is not here, a file here is neither a
workbook, this README nor an AppleScript in `scripts/`, or a workbook still
holds an account's name (in `docProps/core.xml`, `docProps/app.xml`, a
comment's author or a person), an `absPath` or a home folder path. The
script says which; `make fixtures:scrub` removes the ones in
`docProps/core.xml`, the `absPath` and a linked workbook's absolute path.

## The AppleScripts

Each script in [`scripts/`](scripts/) makes its workbook in Excel for Mac:

```bash
osascript fixtures/scripts/table.applescript ~/Library/Containers/com.microsoft.Excel/Data/fixtures/table.xlsx
```

- Excel is sandboxed: it asks for access to a folder outside its container
  before it saves there, and the script waits on that dialog. Save under
  `~/Library/Containers/com.microsoft.Excel/Data/`, then copy the workbook
  here. The folder must exist and must not hold the workbook yet, or Excel
  asks whether to replace it.
- Excel's AppleScript has no `Formula2`: a formula set through `formula` is
  saved as a legacy formula that does not spill, and a table's calculated
  column is not filled. `dynamic-array.applescript` and
  `table.applescript` paste those formulas from the clipboard instead,
  which Excel takes as typed, and restore the clipboard's text after the
  last paste (what else it held, such as an image, is lost).
  Excel comes to the front while they run.
- `external-link.applescript` saves both workbooks in the folder it is
  given, so the link's target is the file name alone. Excel also writes the
  absolute path beside it, which `make fixtures:scrub` removes.

## formulas.xlsx

Written with openpyxl and then opened and saved in Excel for Mac, so the
formulas, the table and the cached values are Excel's. Its sheets are
`Sheet1` (the formulas, the table `T` in `D1:G3` with the columns `a'#`,
`a"b`, `x[y]` and `calc`, and `FILTER` and `SEQUENCE` saved as legacy
formulas that do not spill), `Sheet2`,
`it's` and `a b`; its defined names are `MyLambda`
(`=LAMBDA(x,x*2)`) and `Rate` (`=Sheet1!$A$1`). Before it was added here,
`make fixtures:scrub` emptied `dc:creator` (openpyxl) and
`cp:lastModifiedBy` (the account that saved it in Excel) and removed its
`absPath`.
