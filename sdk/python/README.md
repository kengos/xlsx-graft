# xlsx-graft (Python)

Python bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is
a shared C library (`libxlsx_graft_capi`) loaded through ctypes.

## Reading a sheet's rows

```python
import sys

from xlsx_graft import XlsxReader

with XlsxReader.open("book.xlsx") as book:
    for sheet in book.sheets():
        print(f"{sheet.index} {sheet.name} ({sheet.kind}, {sheet.state})")
    for row in book.sheet("Data").rows():
        print(f"{row.number}: {row.values}")

result = book.result
if result.error:
    print(f"{result.error.code}: {result.error.message}", file=sys.stderr)
for d in result.diagnostics:
    print(f"{d.severity} {d.code}: {d.message}", file=sys.stderr)
```

- `XlsxReader.open_bytes(data)` reads a workbook from its bytes (anything
  bytes-like, copied when called).
- `book.sheet()` takes a name (letter case aside) or an index (from 0,
  hidden sheets and chartsheets counted); the first sheet when given
  neither. To look for a sheet, look in `book.sheets()`: a sheet asked for
  and refused fails the result.
- `rows()` is a generator that reads one row at a time. Only the rows the
  sheet writes come, each with its number from 1 and its values from
  column A, `None` where a column has none.
- The book is opened as the `with` block starts and closed when it ends,
  however it ends, and its rows with it. Using it outside the block raises
  `xlsx_graft.ClosedError`. An exception your block raises is yours: it
  comes out of the `with`, the book closed first.
- `book.result` says how the reading went, once the block has ended. Its
  value is always `None`: to keep the rows read before a problem stopped
  the reading, collect them in a variable outside the block. When an
  exception (Ctrl-C) cut the book's close short, it fails with
  `interrupted`, its diagnostics unread.
- The result is successful when nothing stopped the call and no diagnostic
  is an error, as `xlsx-graft read`'s `ok`. Nothing leaves your block
  early: a file that cannot be opened or an unreadable workbook lists no
  sheets, a sheet refused reads no rows, and `result.error.code` says why
  (`io`, `workbook_unreadable`, `sheet_not_found`, `sheet_ambiguous`,
  `sheet_not_worksheet`, `sheet_unreadable`, ...). A problem inside the
  sheet stops its rows; its diagnostic says what and where.

| Cell | Python |
| --- | --- |
| empty | `None` |
| string | `str` |
| number | `float` |
| boolean | `bool` |
| date | `datetime.date` |
| time of day | `datetime.time` |
| date-time | `datetime.datetime`, naive |
| elapsed time | `datetime.timedelta` |
| error | `xlsx_graft.CellError` (`text`, such as `"#N/A"`) |

A workbook's dates and times are wall-clock times with no zone, and are
given as such: no time zone is applied.

## The engine's version

```python
import sys

import xlsx_graft

result = xlsx_graft.version()
if result.success:
    print(result.value)
else:
    for d in result.diagnostics:
        print(f"{d.code}: {d.message}", file=sys.stderr)
```

Calls return a `xlsx_graft.Result` rather than raising. What raises is a
missing engine library (`xlsx_graft.LibraryNotFoundError`), a book used
outside its block (`xlsx_graft.ClosedError`), and a call made wrongly
(`TypeError` or `ValueError`: a sheet that is neither a name nor an index
from 0, a book entered twice, a result read before the book is closed).

## Where the engine library comes from

The package never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library` argument, as in `xlsx_graft.version(library="/path/to/libxlsx_graft_capi.so")`
   or `XlsxReader.open(path, library=...)`
3. the copy shipped inside the wheel, in `xlsx_graft/native/`

Python 3.11 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Python installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:python
```
