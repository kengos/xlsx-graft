# xlsx_graft (Ruby)

Ruby bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is
a shared C library (`libxlsx_graft_capi`) loaded through fiddle.

## Reading a sheet's rows

```ruby
require "xlsx_graft"

result = XlsxGraft::XlsxReader.open("book.xlsx", timezone: "+09:00") do |book|
  book.sheets.each { |sheet| puts "#{sheet.index} #{sheet.name} (#{sheet.kind}, #{sheet.state})" }
  book.sheet("Data").each_row do |row|
    puts "#{row.number}: #{row.values.inspect}"
  end
end
warn "#{result.error.code}: #{result.error.message}" if result.error
result.diagnostics.each { |d| warn "#{d.severity} #{d.code}: #{d.message}" }
```

- `XlsxReader.open_bytes(string)` reads a workbook from its bytes.
- `book.sheet` takes a name (letter case aside) or an index (from 0, hidden
  sheets and chartsheets counted); the first sheet when given neither. To
  look for a sheet, look in `book.sheets`: a sheet asked for and refused
  fails the result.
- `each_row` reads one row at a time. Only the rows the sheet writes come,
  each with its number from 1 and its values from column A, `nil` where a
  column has none. Without a block it returns an Enumerator.
- The book and its rows are closed when the block ends, however it ends.
  Using them after raises `XlsxGraft::ClosedError`. An exception your
  block raises is yours: it comes out of `open`, the book closed first.
- `result.value` is the block's value on success only; let the block end
  rather than `break` out of it, which returns the break's value instead
  of the result. To keep the rows
  read before a problem stopped the reading, collect them in a variable
  outside the block.
- The result is successful when nothing stopped the call and no diagnostic
  is an error, as `xlsx-graft read`'s `ok`. Nothing leaves your block
  early: a sheet refused reads no rows (an unreadable workbook lists no
  sheets), and `result.error.code` says why (`sheet_not_found`,
  `sheet_ambiguous`, `sheet_not_worksheet`, `sheet_unreadable`,
  `workbook_unreadable`, `io`, ...). A problem inside the sheet stops its
  rows; its diagnostic says what and where.

| Cell | Ruby |
| --- | --- |
| empty | `nil` |
| string | `String` |
| number | `Float` |
| boolean | `true` / `false` |
| date | `Date` |
| time of day | `XlsxGraft::TimeOfDay` (hour, minute, second, millisecond) |
| date-time | `Time` in `timezone:` (UTC by default) |
| elapsed time | `Float` of seconds |
| error | `XlsxGraft::CellError` (`text`, such as `"#N/A"`) |

A workbook's dates and times are wall-clock times with no zone; `timezone:`
takes what `Time.new(in:)` takes (`"UTC"`, `"+09:00"`, a timezone object).
An IANA name such as `"Asia/Tokyo"` is not one of them: pass a timezone
object (`TZInfo::Timezone.get("Asia/Tokyo")`) instead. A date-time the
object's rules cannot place, as one daylight saving skips, stops the read
with `datetime_unrepresentable`.

## The engine's version

```ruby
require "xlsx_graft"

result = XlsxGraft.version
if result.success?
  puts result.value
else
  result.diagnostics.each { |d| warn "#{d.code}: #{d.message}" }
end
```

Calls return a `XlsxGraft::Result` rather than raising. What raises is a
missing engine library (`XlsxGraft::LibraryNotFound`), a book used after
its block (`XlsxGraft::ClosedError`), and a call made wrongly
(`ArgumentError`: no block, a zone `Time` does not take, a sheet that is
neither a name nor an index).

## Where the engine library comes from

The gem never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library:` argument, as in `XlsxGraft.version(library: "/path/to/libxlsx_graft_capi.so")`
   or `XlsxReader.open(path, library: ...)`
3. the copy shipped inside the gem, in `lib/xlsx_graft/native/`

Ruby 3.4 or newer.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Ruby installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:ruby
```
