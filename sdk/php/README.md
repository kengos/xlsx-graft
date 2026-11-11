# xlsx-graft (PHP)

PHP bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is
its CLI, `xlsx-graft`, which the package runs as a subprocess.

## Reading a sheet's rows

```php
use XlsxGraft\Book;
use XlsxGraft\XlsxReader;

$result = XlsxReader::open('book.xlsx', function (Book $book): void {
    foreach ($book->sheets() as $sheet) {
        echo "{$sheet->index} {$sheet->name} ({$sheet->kind->value}, {$sheet->state->value})\n";
    }
    foreach ($book->sheet('Data')->rows() as $row) {
        echo $row->number, ': ', json_encode($row->values), "\n";
    }
}, timezone: 'Asia/Tokyo');
if ($result->error !== null) {
    fwrite(STDERR, "{$result->error->code}: {$result->error->message}\n");
}
foreach ($result->diagnostics as $d) {
    fwrite(STDERR, "{$d->severity} {$d->code}: {$d->message}\n");
}
```

- `timezone` (a name or a `DateTimeZone`) and `cli` (the CLI's path) may
  be left out.
- `XlsxReader::openBytes($bytes, ...)` reads a workbook from its bytes,
  written to a temporary file of the book's own, which only your user may
  read, removed when it closes. When it cannot be written, the result fails
  with `io` and the callback is not called; so does a path holding a NUL
  given to `open`, with `invalid_arg`.
- `$book->sheet()` takes a name (letter case aside) or an index (from 0,
  hidden sheets and chartsheets counted); the first sheet when given
  neither. To look for a sheet, look in `$book->sheets()`: a sheet asked
  for and refused fails the result.
- `rows()` is a generator that reads one row at a time, as the CLI prints
  it. Only the rows the sheet writes come, each with its number from 1 and
  its values from column A, `null` where a column has none. Stopping early
  (a `break`) stops the CLI at the next output it flushes, waited for, and
  is no failure; a run stopped early reports no diagnostics.
- The book and its rows are closed when the callback returns, however it
  returns. Using them after throws a `ClosedException`. An exception your
  callback throws is yours: it comes out of `open`, the book closed first.
- `$result->value` is the callback's value on success only. To keep the
  rows read before a problem stopped the reading, collect them in a
  variable outside the callback.
- The result is successful when nothing stopped the call and no diagnostic
  is an error, as `xlsx-graft read`'s `ok`. Nothing leaves your callback
  early: a sheet refused reads no rows (an unreadable workbook lists no
  sheets), and `$result->error->code` says why (`sheet_not_found`,
  `sheet_ambiguous`, `sheet_not_worksheet`, `sheet_unreadable`,
  `workbook_unreadable`, `exit_status` when the CLI failed, a file that
  cannot be opened among them, ...). A problem inside the sheet stops its
  rows; its diagnostic says what and where.
- Each `sheets()` and each sheet's `rows()` runs the CLI, which opens the
  file again: a file changed in between is read as it is then. The
  problems of opening it, which each run finds again, are kept once.
  A row value this package cannot give (`datetime_unrepresentable`,
  `unknown_value`) stops that run too, so it reports none of them.

| Cell | PHP |
| --- | --- |
| empty | `null` |
| string | `string` |
| number | `float` |
| boolean | `true` / `false` |
| date | `XlsxGraft\CalendarDate` (`year`, `month` from 1, `day`) |
| time of day | `XlsxGraft\TimeOfDay` (`hour`, `minute`, `second`, `millisecond`) |
| date-time | `DateTimeImmutable`, the wall-clock time in `timezone` (UTC by default) |
| elapsed time | `float` of seconds, not wrapped at a day |
| error | `XlsxGraft\CellError` (`text`, such as `"#N/A"`) |

A workbook's dates and times are wall-clock times with no zone, so a
date-time is placed in `timezone`. One the zone cannot place, as one
daylight saving skips or has twice, stops the read with
`datetime_unrepresentable`, where PHP would move it without a word. A date
alone and a time of day are not `DateTimeImmutable`s, which would put them
at a midnight or on a day the workbook never gave; `(string)` of either is
its ISO 8601 form (`2024-01-31`, `18:00:00.000`). An elapsed time is
seconds rather than a `DateInterval`, which cannot be compared and leaves
26 hours as 26 hours or as a day and 2 to the reader.

There is no timeout: stop taking rows to stop a read, which takes effect
at the next output the CLI flushes. PHP's `max_execution_time` does not count the time
spent waiting for the CLI, but on Windows.

## Verifying a workbook

```php
use XlsxGraft\Verifier;

$verifier = new Verifier(policy: ['max_formula_bytes' => 32768]);
$result = $verifier->verify('book.xlsx', targets: ['excel >= 2016'], allowParts: ['macro']);
if ($result->failed()) {
    echo "not verified\n";
}
if ($result->error !== null) {
    fwrite(STDERR, "{$result->error->code}: {$result->error->message}\n");
}
foreach ($result->diagnostics as $d) {
    fwrite(STDERR, "{$d->severity} {$d->code}: {$d->message}\n");
}
```

- `new Verifier(policy: ..., cli: ...)` takes the host's policy, an array of
  the engine's limits, `fail_on`, `deny_scheduled` and `forbid`. Keys are
  snake_case or camelCase, in the arrays inside it too (they are written as
  camelCase); the values are not changed. An empty array is the empty
  policy, and so is leaving it out. The policy is written as JSON when the
  verifier is made, so changing your array afterwards changes nothing, and
  is not checked then: a policy the engine does not take is found by the
  first `verify()`, as `policy_invalid` diagnostics. A policy that cannot be
  written as JSON (INF, NAN, a string that is not UTF-8) throws an
  `InvalidArgumentException`. The CLI is found when the verifier is made
  (`CliNotFoundException`), as [below](#where-the-engine-cli-comes-from).
- `verify($path, ...)` and `verifyBytes($data, ...)` take the caller's rules
  as named arguments, each left out when not needed. Four are arrays of
  strings: `targets` (the versions the workbook must run in), `bestEffort`
  (the versions it should run in; a problem with one is a warning), `allowFunctions` and
  `allowParts` (what the engine refuses by default, to let through); a value
  in one that is not a string throws an `InvalidArgumentException`. `failOn`
  is a string: `'warn'` to fail on a warning too.
- The result is successful when the engine says the workbook is ok, and has
  no value. Its diagnostics are the engine's, each with `decidedBy` and
  `fix` when the engine gives them. `error` is set when a file cannot be
  read (`io`, a workbook that is not there or a temporary file that cannot
  be written) and when the CLI fails (`exit_status`, `invalid_output`); a
  fault of the rules (`rules_invalid`, `target_invalid`) or of the policy
  (`policy_invalid`) is a diagnostic, with no `error`. A path or a rule
  holding a NUL is `invalid_arg`, and the CLI is not run. With `failOn:
  'warn'` (or the policy's `fail_on`) a result fails on a warning alone.
- This package does not check a rule's value: a `failOn` that is not `error`
  or `warn`, or a value that is not UTF-8, is refused by the CLI, and the
  result is an `exit_status` error quoting its message.
- There is no timeout: PHP's `max_execution_time` does not count the time
  spent waiting for the CLI, but on Windows.
- The CLI reads a path, so each `verify()` writes the policy to a temporary
  file, and `verifyBytes()` the bytes too, each readable only by your user
  and removed when the call ends, however it ends.

## The engine's version

```php
use XlsxGraft\XlsxGraft;

$result = XlsxGraft::version();
if ($result->success) {
    echo $result->value, "\n";
} else {
    foreach ($result->diagnostics as $d) {
        fwrite(STDERR, "{$d->code}: {$d->message}\n");
    }
}
```

Calls return a `Result` rather than throwing. What throws is a CLI that is
missing or cannot be run (`CliNotFoundException`), a PHP that cannot start
one included (`proc_open` disabled, or no temporary file for its stderr), a
book used after its callback (`ClosedException`), a negative sheet index,
a zone PHP does not know, a policy given to `Verifier` that cannot be
written as JSON, and a rule that is not a string
(`InvalidArgumentException`).

## Where the engine CLI comes from

The package never downloads anything. It runs the first of:

1. the path in `XLSX_GRAFT_CLI` (an empty value counts as unset)
2. the path passed, as in `XlsxGraft::version('/path/to/xlsx-graft')`
3. `xlsx-graft` (or `xlsx-graft.exe`) on `PATH`

A name with no directory in it, in `XLSX_GRAFT_CLI` or passed, is looked up
on `PATH` by the system.

Under PHP-FPM, whose pools clear the environment by default
(`clear_env = yes`), set `env[XLSX_GRAFT_CLI]` or `env[PATH]` in the pool, or
pass the path.

PHP 8.3 or newer, with `proc_open` enabled. No dependencies, and no
extension beyond what PHP builds in.

## Development

Everything runs in Docker through `make` at the repository root; you need no
PHP installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:php
```
