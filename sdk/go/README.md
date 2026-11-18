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

## Verifying a workbook

```go
verifier, err := xlsxgraft.NewVerifier(map[string]any{"maxFormulaBytes": 32768})
if err != nil {
	log.Fatal(err)
}
result, err := verifier.Verify(ctx, "book.xlsx", xlsxgraft.VerifyRules{
	Targets:    []string{"excel >= 2016"},
	AllowParts: []string{"macro"},
})
if err != nil {
	log.Fatal(err)
}
fmt.Println(result.Success)
for _, d := range result.Diagnostics {
	fmt.Fprintf(os.Stderr, "%s %s: %s\n", d.Severity, d.Code, d.Message)
}
if result.Error != nil {
	fmt.Fprintf(os.Stderr, "%s: %s\n", result.Error.Code, result.Error.Message)
}
```

- `NewVerifier(policy, opts...)` takes the host's limits and rules as a
  `map[string]any`. The keys are the engine's, in camelCase (`maxEntries`,
  `failOn`, `forbid`), and are not changed, so the same policy file as the
  CLI's `--policy` works once `json.Unmarshal` reads it into a map. A `nil` map
  is the engine's defaults. The policy is not checked there: the engine checks
  it at the first `Verify`, and a policy it does not take fails every result
  with `policy_invalid` diagnostics. A map that cannot be written as JSON (NaN,
  an infinity, a channel, a function) is an error of `NewVerifier`, as is a CLI
  that cannot be found (`*CLINotFoundError`, found as `Open` finds it). The
  verifier keeps the JSON of the policy as it is when `NewVerifier` runs. A
  string of the policy that is not UTF-8 is written as U+FFFD (`json.Marshal`
  does), and the engine judges the text it gets.
- `Verify(ctx, path, rules)` and `VerifyBytes(ctx, data, rules)` add the rules
  of the call, a `VerifyRules`, each field optional: `Targets`, `BestEffort`,
  `AllowFunctions`, `AllowParts` (slices of strings) and `FailOn` (`"warn"` to
  fail on a warning too). Nothing is checked here: a fault of the rules is a
  `rules_invalid` or `target_invalid` diagnostic, and the workbook is not
  opened.
- The result is a success when the engine's `ok` is true: no error diagnostic,
  and with `FailOn` `"warn"` (the call's or the policy's) no warning either, so
  a result can fail with no error diagnostic. `result.Value` is always empty;
  `result.Diagnostics` has what was found. `result.Error` is set when a file
  cannot be read (`io`, the workbook or a temporary file), a path or a rule
  holds a NUL (`invalid_arg`, the CLI is not run), or the CLI gave no result
  (`exit_status`, `invalid_output`). A `FailOn` other than `"error"` or
  `"warn"` and a rule that is not UTF-8 are refused by the CLI, so they are an
  `exit_status`.
- Each call writes a temporary file of its own for the policy, and for the data
  of `VerifyBytes`, that only your user may read, and removes it when the call
  returns. Changing `data` once the SDK has written the file has no effect,
  even while the call runs.
- The error beside the result is a `*CLINotFoundError` (the CLI cannot be
  started) or the context's error (the CLI was killed); the result is then
  empty.
- A `Verifier` does not change once it is made: share one between goroutines,
  and call it from them at once.

## Viewing a workbook

```go
viewer, err := xlsxgraft.NewViewer()
if err != nil {
	log.Fatal(err)
}
tree, err := viewer.Tree(ctx, "book.xlsx")
if err != nil {
	log.Fatal(err)
}
if tree.Value != nil {
	for _, part := range tree.Value.Parts {
		fmt.Println(part.Name)
	}
}
grid, err := viewer.DescribeSheet(ctx, "book.xlsx", xlsxgraft.SheetRequest{
	Sheet:   "Data",
	Range:   "A1:H50",
	MaxRows: 20,
})
if err != nil {
	log.Fatal(err)
}
if grid.Value != nil {
	for _, row := range grid.Value.Rows {
		for _, cell := range row.Cells {
			fmt.Println(cell.Ref, cell.Value)
		}
	}
}
```

- `NewViewer(opts...)` takes no policy: a view reads a workbook under the CLI's
  default limits, and gives no text form (that is for the CLI and the MCP
  server). The CLI is found as `Open` finds it; a CLI that cannot be found is a
  `*CLINotFoundError`. A `Viewer` does not change once it is made: share one
  between goroutines, and call it from them at once.
- `Tree(ctx, path)` and `TreeBytes(ctx, data)` list the parts of the package
  and the relationships between them, a `*Tree` of `Tree`, `TreePart`,
  `TreeRelationship` and `TreeTarget`. `DescribeSheet(ctx, path, request)` and
  `DescribeSheetBytes(ctx, data, request)` show the cells of one sheet, a
  `*Grid` of `Grid`, `GridTruncated`, `GridRow` and `GridCell`; its `Sheet` is
  a `*SheetInfo`. Each field is a key of the CLI's JSON, in Go's naming (`ID`),
  a value the JSON may make `null` is a pointer, and a size is a `uint64`.
  A cell's `Value` is a `string`, a `bool`, an `int64` for a number written as
  an integer that fits in an `int64` (`90000`, an elapsed time's milliseconds),
  or a `float64` for any other (`2.0`, and an integer that does not fit). Names and text are made for display (control characters
  removed, long text cut), so they are not exact: read the rows with `Open` for
  exact values.
- A `SheetRequest` has `Sheet` (a name), `SheetIndex` (a `*int`), `Range`,
  `MaxRows` and `MaxColumns`, each optional. A zero value (an empty string, a
  `nil` pointer, `0`) is not passed, so `MaxRows: 0` is the CLI's default of
  100 rows. This package checks a value only for a NUL (`invalid_arg`); the CLI
  judges the rest. A request the CLI refuses (a range that is
  not an A1 range, a count out of its bounds, a name and an index together) is
  an `*InvalidRequestError` beside an empty result, whose `Message` is the
  CLI's reason, naming the CLI's options (`--range`).
- The result is a success when the engine's `ok` is true, and its `Value` is
  there also when it is not: a workbook that cannot be read gives a `*Tree` of
  the parts that were read, perhaps none, and a sheet that is not found gives
  a `*Grid` whose `Sheet` is `nil`.
  `Value` is `nil` only when this package did not get one: `result.Error` is
  `exit_status`, `invalid_output`, `invalid_arg` (a path, a sheet name or a
  range holds a NUL, the CLI is not run), or `io` for a temporary file that
  could not be written. The zero result returned beside a non-nil error also
  has a `nil` `Value`. A file that cannot be opened is an `io` error with a
  `Value`.
- A path that is not UTF-8 is given to the CLI as it is. The `Bytes` calls
  write the data to a temporary file of the call's own, that only your user
  may read, and remove it when the call returns.
- The error beside the result is a `*CLINotFoundError` (the CLI cannot be
  started), the context's error (the CLI was killed) or an
  `*InvalidRequestError` (`DescribeSheet` and `DescribeSheetBytes` only); the
  result is then empty.

Go 1.26 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Go installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:go
```
