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

## Verifying a workbook

```csharp
using System.Text.Json.Nodes;
using XlsxGraft;

var verifier = new Verifier(new JsonObject { ["maxFormulaBytes"] = 32768, ["failOn"] = "warn" });
var result = verifier.Verify("book.xlsx", targets: ["excel >= 2016"], allowParts: ["macro"]);
Console.WriteLine(result.Success ? "ok" : "not ok");
foreach (var d in result.Diagnostics)
{
    Console.Error.WriteLine($"{d.Severity} {d.Code}: {d.Message}");
}

if (result.Error is { } error)
{
    Console.Error.WriteLine($"{error.Code}: {error.Message}");
}
```

- `new Verifier(policy)` takes the host's limits and rules as a
  `System.Text.Json.Nodes.JsonObject`, with the engine's camelCase keys, which
  are not changed. The same policy file as the CLI's `--policy` is
  `JsonNode.Parse(File.ReadAllText("policy.json"))!.AsObject()`. The engine
  checks it there: a policy it does not take throws
  `XlsxGraft.PolicyInvalidException`, whose `Diagnostics` say what is wrong
  (for each key, a `policy_invalid` diagnostic).
- `Verify(path, ...)` and `VerifyBytes(data, ...)` take the rules of the call as
  named arguments, each optional: `targets`, `bestEffort`, `allowFunctions`,
  `allowParts` (any `IEnumerable<string>`) and `failOn` (`"warn"` to fail on a
  warning too). `VerifyBytes` pins `data` for the call and does not copy it,
  so it must not change meanwhile.
- The result is successful when the engine's `ok` is true: no error
  diagnostic, and with `failOn: "warn"` (the call's or the policy's) no warning
  either, so such a result can fail with no error diagnostic. A verify result
  has no value; `result.Diagnostics` has what was found. `result.Error` is set
  when the file cannot be read (`io`) or the call itself failed (`panic`,
  `null_arg`, `unknown_status`, `diagnostics_unreadable`). A fault of the rules
  is a `rules_invalid` or `target_invalid` diagnostic, and the workbook is not
  opened.
- A policy and rules that make more than 2 MiB of JSON, or a policy that holds
  a value JSON cannot write (`double.NaN`, a `Type`, nesting too deep), throw
  `ArgumentException`; a `null` path throws `ArgumentNullException`. For a path
  that is not Unicode (a lone surrogate), the package reads the file itself: if
  it cannot, the result is the `io` error and the rules are not checked.
- A verifier does not change once it is made: share one between threads, and
  call it from them at once.

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
missing engine library (`XlsxGraft.LibraryNotFoundException`), a verifier
policy the engine does not take (`XlsxGraft.PolicyInvalidException`), a book used
after it is disposed (`ObjectDisposedException`), and a call made wrongly
(`ArgumentNullException`, `ArgumentOutOfRangeException` for an index below
0, `InvalidOperationException` for a result read before the book is
disposed).

## Where the engine library comes from

The package never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library` argument, as in `Engine.Version(library: "/path/to/libxlsx_graft_capi.so")`
   or `XlsxReader.Open(path, library: ...)`
3. the library the package ships: in `runtimes/<rid>/native/` beside the
   application (NuGet copies the one for the machine's runtime identifier into a
   build's output), or beside the application itself (what a publish for one
   runtime writes)

The libraries are built for glibc Linux, macOS and Windows x64; on Alpine (musl)
the library is found and fails to load, and the error says so.

.NET 10 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
.NET installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:dotnet
```
