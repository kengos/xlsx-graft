# xlsx-graft (Node.js)

Node.js bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is a
Node.js addon (`xlsx_graft.node`) that links its C ABI in.

## Reading a sheet's rows

```js
import { XlsxReader } from 'xlsx-graft';

const result = XlsxReader.open('book.xlsx', { timezone: 'Asia/Tokyo' }, (book) => {
  for (const sheet of book.sheets()) {
    console.log(`${sheet.index} ${sheet.name} (${sheet.kind}, ${sheet.state})`);
  }
  for (const row of book.sheet('Data').rows()) {
    console.log(row.number, row.values);
  }
});
if (result.error) console.error(`${result.error.code}: ${result.error.message}`);
for (const d of result.diagnostics) console.error(`${d.severity} ${d.code}: ${d.message}`);
```

- `XlsxReader.openBytes(uint8Array, ...)` reads a workbook from its bytes,
  copied when called. `open` takes a path or a `file:` URL. The options
  (`timezone`, `addon`) may be left out: `XlsxReader.open(path, (book) => ...)`.
- `book.sheet` takes a name (letter case aside) or an index (from 0, hidden
  sheets and chartsheets counted); the first sheet when given neither. To
  look for a sheet, look in `book.sheets()`: a sheet asked for and refused
  fails the result.
- `rows()` is a synchronous iterator that reads one row at a time. Only the
  rows the sheet writes come, each with its number from 1 and its values
  from column A, `null` where a column has none.
- The book and its rows are closed when the callback returns, however it
  returns. Using them after throws a `ClosedError`. An exception your
  callback throws is yours: it comes out of `open`, the book closed first.
  The callback must be synchronous: one that returns a promise throws a
  `TypeError`, the book already closed.
- `result.value` is the callback's value on success only. To keep the rows
  read before a problem stopped the reading, collect them in a variable
  outside the callback.
- The result is successful when nothing stopped the call and no diagnostic
  is an error, as `xlsx-graft read`'s `ok`. Nothing leaves your callback
  early: a sheet refused reads no rows (an unreadable workbook lists no
  sheets), and `result.error.code` says why (`sheet_not_found`,
  `sheet_ambiguous`, `sheet_not_worksheet`, `sheet_unreadable`,
  `workbook_unreadable`, `io`, ...). A file that cannot be opened does not
  call the callback. A problem inside the sheet stops its rows; its
  diagnostic says what and where.

| Cell | JavaScript |
| --- | --- |
| empty | `null` |
| string | `string` |
| number | `number` |
| boolean | `true` / `false` |
| date | `CalendarDate` (`year`, `month` from 1, `day`) |
| time of day | `TimeOfDay` (`hour`, `minute`, `second`, `millisecond`) |
| date-time | `Date`, the wall-clock time in `timezone` (UTC by default) |
| elapsed time | `number` of milliseconds |
| error | `CellError` (`text`, such as `"#N/A"`) |

A workbook's dates and times are wall-clock times with no zone. A `Date` is
an instant, so a date-time is placed in `timezone`: any zone
`Intl.DateTimeFormat` takes, an IANA name (`"Asia/Tokyo"`) or an offset
(`"+09:00"`). A date-time the zone cannot place, as one daylight saving
skips or has twice, stops the read with `datetime_unrepresentable`. A date
alone and a time of day are not `Date`s, which would put them at a midnight
or on a day the workbook never gave; `String()` of either is its ISO 8601
form (`2024-01-31`, `18:00:00.000`).

## Verifying a workbook

```js
import { Verifier } from 'xlsx-graft';

const verifier = new Verifier({ policy: { maxFormulaBytes: 32768, failOn: 'warn' } });
const result = verifier.verify('book.xlsx', {
  targets: ['excel >= 2016'],
  allowParts: ['macro'],
});
console.log(result.success ? 'ok' : 'not ok');
for (const d of result.diagnostics) console.error(`${d.severity} ${d.code}: ${d.message}`);
if (result.error) console.error(`${result.error.code}: ${result.error.message}`);
```

- `new Verifier({ policy })` takes the host's limits and rules as an object,
  its keys as the engine names them (camelCase; none is renamed, so the same
  policy file as the CLI's `--policy`, read with `JSON.parse`, works). The
  engine checks it there: a policy it does not take throws a
  `PolicyInvalidError`, whose `diagnostics` say what is wrong (for each key, a
  `policy_invalid` diagnostic).
- `verify(path, rules)` and `verifyBytes(uint8Array, rules)` take the rules of
  the call as an object, each key optional: `targets`, `bestEffort`,
  `allowFunctions`, `allowParts` (arrays of strings) and `failOn` (`'warn'` to
  fail on a warning too). `verify` takes a path or a `file:` URL.
  `verifyBytes` reads the array in place for the call, not copied.
- The result is successful when the engine's `ok` is true: no error
  diagnostic, and with `failOn: 'warn'` (the call's or the policy's) no
  warning either, so such a result can fail with no error diagnostic.
  `result.value` is always `null`; `result.diagnostics` has what was found.
  `result.error` is set when the file cannot be read (`io`) or the call
  itself failed (`panic`, `null_arg`, `unknown_status`,
  `diagnostics_unreadable`). A fault of the rules is a `rules_invalid` or
  `target_invalid` diagnostic, and the workbook is not opened.
- A policy and rules that make more than 2 MiB of JSON throw a `RangeError`;
  a value JSON cannot write (a BigInt, a cycle) throws a `TypeError`. For a
  path with a lone surrogate, the package reads the file itself (Node.js opens
  the name with U+FFFD in place of the lone surrogate): if it cannot, the
  result is the `io` error and the rules are not checked.
- A lone surrogate in a string of the policy or the rules is written as
  U+FFFD, and the engine checks that string as it checks any other (a
  `policy_invalid`, `target_invalid` or `rules_invalid` diagnostic).
- A verifier does not change once it is made. A worker thread makes its own.

## The engine's version

```js
import { version } from 'xlsx-graft';

const result = version();
if (result.success) {
  console.log(result.value);
} else {
  for (const d of result.diagnostics) {
    console.error(`${d.code}: ${d.message}`);
  }
}
```

Calls return a `Result` rather than throwing. What throws is a missing
engine addon (`LibraryNotFoundError`), a book used after its callback
(`ClosedError`), a policy the engine does not take (`PolicyInvalidError`), a
call this package built wrongly, which the engine refuses (`XlsxGraftError`: a
bug of the package), and a call made wrongly (a `TypeError` or a `RangeError`:
no callback, a zone `Intl` does not take, a sheet that is neither a name
nor an index, a policy and rules of more than 2 MiB of JSON, a value JSON
cannot write, a URL that is not a `file:` URL).

The package is ESM; `require('xlsx-graft')` loads it as well, through
Node.js's `require()` of ES modules.

## Where the engine addon comes from

The package never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_ADDON` (an empty value counts as unset)
2. the `addon` option, as in `version({ addon: '/path/to/xlsx_graft.node' })`
3. the addon of this platform's package, `@xlsx-graft/<os>-<cpu>[-gnu|-msvc]`,
   which npm installs with this package (as an optional dependency) on Linux
   (glibc) x64 and arm64, macOS x64 and arm64, and Windows x64

Node.js 24 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Node.js installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:js
```
