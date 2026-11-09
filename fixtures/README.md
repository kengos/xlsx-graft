# fixtures

Workbooks saved by Excel, for tests and manual checks that need a workbook
Excel wrote rather than one the tests build. Each has one row below: what it
is for, the Excel that saved it, how it was made, and what
`make excel:check` saw when Excel opened it.

The engine's tests read every workbook here and compare what reading and
verifying it gives with the lines written in
[`engine/verify/tests/fixtures/expected.rs`](../engine/verify/tests/fixtures/expected.rs).

| File | What it is for | Saved by | How it was made | `make excel:check` |
| --- | --- | --- | --- | --- |
| `chart.xlsx` | A clustered column chart (chart and drawing parts) | Excel for Mac 16.114.4 | [`scripts/chart.applescript`](scripts/chart.applescript) | No repair, no dialog (2026-11-06) |
| `date1904.xlsx` | The 1904 date system (`workbookPr@date1904`), dates and a date-time under date formats | Excel for Mac 16.114.4 | [`scripts/date1904.applescript`](scripts/date1904.applescript) | No repair, no dialog (2026-11-06) |
| `dynamic-array.xlsx` | Formulas that spill (`SEQUENCE`, `SORT`, `FILTER`, `UNIQUE`) and a spill reference (`A1#`) | Excel for Mac 16.114.4 | [`scripts/dynamic-array.applescript`](scripts/dynamic-array.applescript) | No repair, no dialog (2026-11-06) |
| `external-link.xlsx` | Formulas that refer to another workbook, `external-source.xlsx` (an external workbook reference part) | Excel for Mac 16.114.4 | [`scripts/external-link.applescript`](scripts/external-link.applescript) | No repair; Excel asks whether to update the link, answered "Don't Update" (2026-11-06) |
| `external-source.xlsx` | The workbook `external-link.xlsx` refers to | Excel for Mac 16.114.4 | [`scripts/external-link.applescript`](scripts/external-link.applescript) | No repair, no dialog (2026-11-06) |
| `formulas.xlsx` | Formulas the formula lexer was checked against: defined names, `LAMBDA`, a table's structured references, quoted sheet names, `WEBSERVICE` | Excel for Mac 16 (the release was not recorded) | [Below](#formulasxlsx) | No repair, no dialog (2026-11-06) |
| `macro.xlsm` | A VBA project (`xl/vbaProject.bin`): the module `Module1` with `Sub AddOne`, which adds 1 to `Sheet1!A1` (1 as saved) | Excel for Mac 16 (the release was not recorded) | By hand in Excel ([below](#macroxlsm)) | No repair; Excel asks whether to disable the macros, answered "Disable Macros" (2026-11-09) |
| `pivot.xlsx` | A pivot table over a table (pivot cache definition, pivot cache records and pivot table parts), with the labels Excel's Japanese user interface gave it | Excel for Mac 16 (the release was not recorded) | By hand in Excel ([below](#pivotxlsx)) | No repair, no dialog (2026-11-09) |
| `python-in-excel.xlsm` | Python in Excel: a `PY` formula, `xl/python.xml`, a cell image in rich data, an Office Script named in `customXml/`, and the macro-enabled content type with no VBA project | Excel for Mac 16 (the release was not recorded), then edited by hand | Excel's Python in Excel sample, edited ([below](#python-in-excelxlsm)) | No repair, no dialog (2026-11-09) |
| `table.xlsx` | A table with a calculated column and a totals row, and a formula outside it that refers to a column of it | Excel for Mac 16.114.4 | [`scripts/table.applescript`](scripts/table.applescript) | No repair, no dialog (2026-11-06) |

Every workbook went through `make fixtures:scrub` before it was added
([Adding a workbook](#adding-a-workbook)). "No repair" means
`make excel:check` gave `repair_dialog: false` and
`error: null`; "no dialog" that it also gave `dialogs: []`.

## Adding a workbook

1. Save it in Excel, with neutral sheet names and values (`Sheet1`, `Data`,
   numbers and English words). The names Excel gives in another language,
   such as the labels of a pivot table, may stay (see
   [pivot.xlsx](#pivotxlsx)); text a person wrote may not.
2. Copy it here and run `make fixtures:scrub F=fixtures/<name>.xlsx`. Excel
   writes the account's name into `docProps/core.xml` (`dc:creator`,
   `cp:lastModifiedBy`) and `docProps/app.xml` (`Company`, `Manager`), the
   folder it saved the workbook in (`x15ac:absPath`) into the workbook
   part, the absolute path of a linked workbook into an external link
   (`xxl21:alternateUrls`, with the relationships only those URLs name),
   and the account that last refreshed a pivot cache into its definition
   (`refreshedBy`); the script empties or removes all of them and leaves
   the rest as Excel wrote it.
3. Read, for names, accounts and paths, everything
   [What the check does not read](#what-the-check-does-not-read) lists.
4. Run `make excel:check F=fixtures/<name>.xlsx` on a Mac with Excel and
   note what it saw.
5. Add the workbook's row above, and its entry to
   `engine/verify/tests/fixtures/expected.rs` in file-name order (the test
   that compares the entries with the workbooks fails on any other order).

`make engine:budget` (part of `make engine:verify`) runs
`scripts/check-fixtures.sh`, which fails when

- a workbook here has no row above, or a row names a workbook that is not
  here;
- a file here is a symbolic link, or is neither a workbook directly in this
  folder, this README nor an AppleScript directly in `scripts/` (a Finder
  `.DS_Store` directly in this folder or in `scripts/` that git does not
  track aside; without git, or outside a git work tree, any `.DS_Store`
  fails), or this README or an AppleScript holds a home folder path;
- a workbook holds a part of a kind the script does not list, such as
  `docProps/custom.xml`, comments, persons or `docMetadata/LabelInfo.xml`,
  where Excel writes accounts;
- a workbook's `dc:creator`, `cp:lastModifiedBy`, `Company`, `Manager` or
  `HyperlinkBase` is not empty, or a part holds `absPath`,
  `alternateUrls`, `refreshedBy`, `fileSharing`, `securityDescriptor` or a
  home folder path;
- an external relationship's target is not a file name alone (a linked
  workbook in this folder), but a URL or a path;
- a workbook's Office Script list (`customXml/item1.xml`, with its
  properties) is not the one with the placeholder IDs
  [python-in-excel.xlsm](#python-in-excelxlsm) describes;
- a workbook holds a VBA project (`xl/vbaProject.bin`) and is not
  `macro.xlsm`, or the project's SHA-256 is not the one of the project
  [macro.xlsm](#macroxlsm) describes, the only part here that holds a home
  folder path.

`scripts/check-fixtures.sh` prints each condition a workbook fails, with
the workbook's file name. `make fixtures:scrub` empties or removes the
fields in step 2 and `HyperlinkBase`. A part of another kind is removed
from the workbook (with its relationship and its content type), or, once a
person has read it and found no account in it, added to the kinds the
script lists. A `fileSharing` element, a `securityDescriptor`, an external
target and an Office Script list are removed or replaced by hand.

### What the check does not read

`scripts/check-fixtures.sh` covers the fields above and nothing more. It
is not meant to find every place a workbook can hold a name; the person
who adds a workbook reads the rest (step 3). The script does not read:

- the text of cells, shapes and charts;
- the names, titles and alternative text of shapes, pictures and charts
  (`xdr:cNvPr`; Excel fills the alternative text with an inserted
  picture's file name);
- hyperlinks within the workbook, the file names its links name (a link
  to a URL or a path fails the check), and the hyperlink targets Office
  lists in `docProps/app.xml` (`HLinks`);
- the text and metadata inside images;
- headers and footers (a header that shows the user name has Excel write
  that name);
- document properties: title, subject, keywords, comments, category,
  content status and identifier, and, in `docProps/app.xml`, the template
  and a digital signature;
- sheet, table and defined names (sheet and defined names are also
  listed in `docProps/app.xml`'s `TitlesOfParts`), pivot fields and data
  validation messages;
- rich values: a linked data type (stocks, geography, an organization's
  data) names its provider;
- the names of custom views (`customWorkbookView`), which Excel may fill
  with the user name when the workbook was shared;
- the strings inside formulas, and the Python code in `PY` formulas and
  `xl/python.xml`;
- the names in the theme and styles, and the text inside number formats;
- the paths in DDE links and web publishing items, which are not
  relationships;
- the phonetic readings of strings (`rPh`), which can hold a person's
  name;
- a home folder path other than a macOS or Windows one (`/home/` on
  Linux);
- a part edited by hand, and a name written in a form Excel does not
  write when it saves, such as a UTF-16 part or a percent-encoded path. A
  VBA project, where Excel writes paths in UTF-16 and in a compressed
  stream, is not searched but compared whole with the one
  [macro.xlsm](#macroxlsm) describes.

A part where Excel writes an account, such as `docMetadata/LabelInfo.xml`
(a sensitivity label) or `docProps/custom.xml`, is not of a kind the
script lists, so the script fails on it without reading it.

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

## macro.xlsm

Made by hand in Excel for Mac with a Japanese user interface: `1` in
`Sheet1!A1`, and in the VBA editor the standard module `Module1` holding

```vba
Sub AddOne()
    Range("A1").Value = Range("A1").Value + 1
End Sub
```

The project has no UserForm. Excel's AppleScript cannot make a VBA
project, so no script here makes this workbook.

The VBA project names the type libraries Excel for Mac gives every
project: Visual Basic for Applications and the Microsoft Excel 16.0 Object
Library by their paths under `/Applications/Microsoft Excel.app`, the
Microsoft Office 16.0 Object Library (`mso.tlb`), and Microsoft Forms 2.0
Object Library, once as `fm20.tlb` and once as a path in the account's
home folder,

```text
~/Library/Containers/com.microsoft.Excel/Data/Library/Preferences/MSForms.exd
```

written with the home folder's full path, in UTF-16 in the `_VBA_PROJECT`
stream and in ASCII in the `dir` stream, which is compressed, of
`xl/vbaProject.bin`. The VBA editor does not let this reference be
removed (it answers that the control or reference is in use and cannot be
deleted), so a VBA project saved by Excel for Mac holds that path, with
the account's name in it. It is the only home folder path in the workbooks
here.

`scripts/check-fixtures.sh` cannot read a path in UTF-16 or in a
compressed stream, so instead of searching the project it compares its
SHA-256 with that of the project read for this README: the module above,
the references above, and no other path or name. It fails on a VBA project
in any other workbook, and on any other project in this one; a project
saved again is read again and its SHA-256 replaces the one in the script.
The project was read as its `dir` stream decompressed, its module
sources, its `PROJECT` stream, and the ASCII and UTF-16 strings of the
whole part.

`make fixtures:scrub` emptied `dc:creator` and `cp:lastModifiedBy` and
removed the `absPath`; it does not change the VBA project, and does not
search it for the home folder as it searches the other parts.

## pivot.xlsx

Made by hand in Excel for Mac with a Japanese user interface. The sheet
`Data` holds a table (`Item`, `Qty`, `Price`, four rows); the sheet `Pivot`
holds a pivot table made from it with `Item` as rows and the sum of `Qty`
as values. The table, the pivot table, the row label header, the grand
total and the data field carry the Japanese names Excel gave them; the
pivot cache's source is the table by its Japanese name, and verify pushes
no diagnostic for it. `make fixtures:scrub`
emptied `dc:creator` and `cp:lastModifiedBy`, removed the `absPath` and
the pivot cache's `refreshedBy` (the account that refreshed it).

## python-in-excel.xlsm

Made from Excel's Python in Excel sample in Excel for Mac with a Japanese
user interface and saved as `.xlsm`: the Iris data set as the table
`IrisDataSet2` on the sheet `Python`, a `PY` formula in `G3` that plots
two of its columns, the plot as a floating image, and a text box over it.
The last row of the sheet credits the data set's source (the Iris data
set, CC BY 4.0). The sheet `Sheet1`, with 1 in `A1`, was in the workbook
as it was saved.

The sample's explanatory text was not kept: the text box and the cell
above the plot hold text written for this workbook instead. What remains
of the sample is the data set, the five lines of `matplotlib` in the `PY`
formula, and the setup code Excel writes into every workbook with Python
(`xl/python.xml`).

Before it was added here, its entries were rewritten by hand, each change
replacing text and nothing else:

- The Japanese sheet name became `Python`, in `xl/workbook.xml`,
  `docProps/app.xml` and the image's formula in `xl/drawings/drawing1.xml`.
- The Japanese text was replaced: the cell above the table became its
  English title, the cell above the plot and the text box became the text
  written for this workbook, and the text box's name and the provider
  label in `xl/richData/rdsupportingpropertybag.xml` became English.
- `customXml/item1.xml` names the Office Script attached to the workbook by
  its OneDrive item ID and its sharing link (a base64-encoded URL), which
  named a tenant and an account. Both were replaced by placeholders of the
  same form: `01` and `A`s to the ID's length, and the base64url of a URL
  under `https://example-my.sharepoint.com/`; `scripts/check-fixtures.sh`
  fails on any other, and on another properties part
  (`customXml/itemProps1.xml`, whose GUID it also compares). Excel opens
  the workbook without a repair, so the part was kept.

Then `make fixtures:scrub` emptied `dc:creator` and `cp:lastModifiedBy` and
removed the `absPath`.
