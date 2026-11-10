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
(`ClosedError`), and a call made wrongly (a `TypeError` or a `RangeError`:
no callback, a zone `Intl` does not take, a sheet that is neither a name
nor an index).

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
