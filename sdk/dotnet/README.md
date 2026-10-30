# XlsxGraft (.NET)

.NET bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is a
shared C library (`libxlsx_graft_capi`) called through unmanaged function
pointers.

## Reading a sheet's rows

```csharp
using XlsxGraft;

var book = XlsxReader.Open("book.xlsx");
using (book)
{
    foreach (var sheet in book.Sheets())
    {
        Console.WriteLine($"{sheet.Index} {sheet.Name} ({sheet.Kind}, {sheet.State})");
    }

    foreach (var row in book.Sheet("Data").Rows())
    {
        Console.WriteLine($"{row.Number}: {string.Join(", ", row.Values)}");
    }
}

var result = book.Result;
if (result.Error is { } error)
{
    Console.Error.WriteLine($"{error.Code}: {error.Message}");
}

foreach (var d in result.Diagnostics)
{
    Console.Error.WriteLine($"{d.Severity} {d.Code}: {d.Message}");
}
```

- `XlsxReader.OpenBytes(data)` reads a workbook from its bytes (copied
  when called).
- `book.Sheet()` takes a name (letter case aside) or an index (from 0,
  hidden sheets and chartsheets counted); the first sheet when given
  neither. To look for a sheet, look in `book.Sheets()`: a sheet asked for
  and refused fails the result.
- `Rows()` reads one row at a time as it is enumerated. Only the rows the
  sheet writes come, each with its number from 1 and its values from
  column A, `null` where a column has none.
- The book is opened when `Open` returns and closed when its `using` block
  ends, however it ends, and its rows with it. Using it after raises
  `ObjectDisposedException`. An exception your block throws is yours: it
  comes out of the `using`, the book closed first.
- `book.Result` says how the reading went, once the book is disposed. It
  has no value: to keep the rows read before a problem stopped the
  reading, collect them in a variable outside the block.
- The result is successful when nothing stopped the call and no diagnostic
  is an error, as `xlsx-graft read`'s `ok`. Nothing leaves your block
  early: a file that cannot be opened or an unreadable workbook lists no
  sheets, a sheet refused reads no rows, and `result.Error.Code` says why
  (`io`, `workbook_unreadable`, `sheet_not_found`, `sheet_ambiguous`,
  `sheet_not_worksheet`, `sheet_unreadable`, ...). A problem inside the
  sheet stops its rows; its diagnostic says what and where.
- One book may be read from several threads: every call takes the book's
  lock.

| Cell | .NET |
| --- | --- |
| empty | `null` |
| string | `string` |
| number | `double` |
| boolean | `bool` |
| date | `DateOnly` |
| time of day | `TimeOnly` |
| date-time | `DateTime`, `Kind = Unspecified` |
| elapsed time | `TimeSpan` |
| error | `XlsxGraft.CellError` (`Text`, such as `"#N/A"`) |

A workbook's dates and times are wall-clock times with no zone, and are
given as such: no time zone is applied.

## The engine's version

```csharp
var result = XlsxGraft.Engine.Version();
if (result.Success)
{
    Console.WriteLine(result.Value);
}
else
{
    foreach (var d in result.Diagnostics)
    {
        Console.Error.WriteLine($"{d.Code}: {d.Message}");
    }
}
```

Calls return a `XlsxGraft.Result` rather than throwing. What throws is a
missing engine library (`XlsxGraft.LibraryNotFoundException`), a book used
after it is disposed (`ObjectDisposedException`), and a call made wrongly
(`ArgumentNullException`, `ArgumentOutOfRangeException` for an index below
0, `InvalidOperationException` for a result read before the book is
disposed).

## Where the engine library comes from

The package never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library` argument, as in `Engine.Version(library: "/path/to/libxlsx_graft_capi.so")`
   or `XlsxReader.Open(path, library: ...)`
3. a library in `native/` beside the application (no package ships one yet)

.NET 10 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
.NET installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:dotnet
```
