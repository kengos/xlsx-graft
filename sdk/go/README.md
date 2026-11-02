# xlsx-graft (Go)

Go bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an engine
that reads, verifies, creates and edits xlsx workbooks. The engine is its CLI,
`xlsx-graft`, which the package runs as a subprocess.

## Reading a sheet's rows

```go
import xlsxgraft "github.com/kengos/xlsx-graft/sdk/go"

tokyo, _ := time.LoadLocation("Asia/Tokyo")
result, err := xlsxgraft.Open(ctx, "book.xlsx", func(book *xlsxgraft.Book) (int, error) {
	sheets, err := book.Sheets()
	if err != nil {
		return 0, err
	}
	for _, s := range sheets {
		fmt.Printf("%d %s (%s, %s)\n", s.Index, s.Name, s.Kind, s.State)
	}
	rows := book.Sheet("Data").Rows()
	defer rows.Close()
	count := 0
	for rows.Next() {
		row := rows.Row()
		fmt.Println(row.Number, row.Values)
		count++
	}
	return count, rows.Err()
}, xlsxgraft.WithLocation(tokyo))
if err != nil {
	log.Fatal(err)
}
if result.Error != nil {
	fmt.Fprintf(os.Stderr, "%s: %s\n", result.Error.Code, result.Error.Message)
}
for _, d := range result.Diagnostics {
	fmt.Fprintf(os.Stderr, "%s %s: %s\n", d.Severity, d.Code, d.Message)
}
```

- `WithLocation` (UTC by default) and `WithCLI` (the CLI's path) may be
  left out.
- `xlsxgraft.OpenBytes(ctx, data, ...)` reads a workbook from its bytes,
  written to a temporary file of the book's own, which only your user may
  read, removed when it closes. When it cannot be written, the result fails
  with `io` and the function is not called; so does a path holding a NUL
  given to `Open`, with `invalid_arg`.
- `book.Sheet(name)` takes a name (letter case aside), `book.SheetAt(index)`
  an index (from 0, hidden sheets and chartsheets counted); `SheetAt(0)` is
  the first sheet. To look for a sheet, look in `book.Sheets()`: a sheet
  asked for and refused fails the result.
- `Rows` is pulled as a `bufio.Scanner` is (`Next`, `Row`, `Err`), one row at
  a time, as the CLI prints it. Only the rows the sheet writes come, each
  with its number from 1 and its values from column A, `nil` where a column
  has none. Stopping early (`Close`) stops the CLI at the next output it
  flushes, waited for, and is no failure; a run stopped early reports no
  diagnostics.
- The context cancels: once it is done, the CLI running is killed (it
  leaves no file behind: the engine spools to unnamed temporary files), and
  `Rows.Err` and `Open` return its error. That is the only timeout.
- The book and its rows are closed when the function returns, however it
  returns. Using them after gives `ErrClosed`. The error your function
  returns is `Open`'s error, the result then the zero `Result`; so is the
  first error the book gave it, even if it went on (a CLI that cannot be
  started, the context done, a negative index). A panic propagates, the
  book closed first.
- `result.Value` is the function's value on success only. To keep the rows
  read before a problem stopped the reading, collect them in a variable
  outside the function.
- The result is successful when nothing stopped the call and no diagnostic
  is an error, as `xlsx-graft read`'s `ok`. Nothing leaves your function
  early: a sheet refused reads no rows (an unreadable workbook lists no
  sheets), and `result.Error.Code` says why (`sheet_not_found`,
  `sheet_ambiguous`, `sheet_not_worksheet`, `sheet_unreadable`,
  `workbook_unreadable`, `exit_status` when the CLI failed, a file that
  cannot be opened among them, ...). A problem inside the sheet stops its
  rows; its diagnostic says what and where.
- The first `Sheets()` (kept for the next) and each sheet's `Rows()` runs
  the CLI, which opens the file again: a file changed in between is read as it is then. The
  problems of opening it, which each run finds again, are kept once. A row
  value this package cannot give (`datetime_unrepresentable`,
  `unknown_value`) stops that run too, so it reports none of them.
- A `Book` may be used from several goroutines; each `Rows` from one at a
  time.

| Cell | Go |
| --- | --- |
| empty | `nil` |
| string | `string` |
| number | `float64` |
| boolean | `bool` |
| date | `xlsxgraft.Date` (`Year`, `Month`, `Day`) |
| time of day | `xlsxgraft.TimeOfDay` (`Hour`, `Minute`, `Second`, `Millisecond`) |
| date-time | `time.Time`, the wall-clock time in `WithLocation` (UTC by default) |
| elapsed time | `time.Duration`, not wrapped at a day |
| error | `xlsxgraft.CellError` (`Text`, such as `"#N/A"`) |

A workbook's dates and times are wall-clock times with no zone, so a
date-time is placed in the location given. One the location cannot place,
as one daylight saving skips or has twice, stops the read with
`datetime_unrepresentable`, where `time.Date` would move it without a word;
so does an elapsed time beyond a `time.Duration`'s 292 years. A date alone
and a time of day are not `time.Time`s, which would put them at a midnight
or on a day the workbook never gave; each prints in its ISO 8601 form
(`2024-01-31`, `18:00:00.000`).

## The engine's version

```go
result, err := xlsxgraft.Version()
if err != nil {
	log.Fatal(err)
}
if result.Success {
	fmt.Println(result.Value)
} else {
	for _, d := range result.Diagnostics {
		fmt.Fprintf(os.Stderr, "%s: %s\n", d.Code, d.Message)
	}
}
```

Calls return a `Result` rather than an error. The errors `Open` returns
are your function's own, a CLI that is missing or cannot be started
(`*CLINotFoundError`), a context done and a negative sheet index; a book
used after its function gives `ErrClosed`, which `Open` never returns.

## Where the engine CLI comes from

The package never downloads anything. It runs the first of:

1. the path in `XLSX_GRAFT_CLI` (an empty value counts as unset)
2. the path passed, as in `xlsxgraft.Version(xlsxgraft.WithCLI("/path/to/xlsx-graft"))`
3. `xlsx-graft` (or `xlsx-graft.exe`) on `PATH`

A name with no directory in it, in `XLSX_GRAFT_CLI` or passed, is looked up
on `PATH` by `os/exec`. On Windows the CLI is an `.exe`: one with no
extension, set or passed, gets `.exe` added.

Go 1.26 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Go installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:go
```
