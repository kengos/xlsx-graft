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

## Verifying a workbook

```ruby
require "xlsx_graft"

verifier = XlsxGraft::Verifier.new(policy: { max_formula_bytes: 32_768, fail_on: "warn" })
result = verifier.verify("book.xlsx", targets: ["excel >= 2016"], allow_parts: ["macro"])
puts(result.success? ? "ok" : "not ok")
result.diagnostics.each { |d| warn "#{d.severity} #{d.code}: #{d.message}" }
warn "#{result.error.code}: #{result.error.message}" if result.error
```

- `Verifier.new(policy:)` takes the host's limits and rules as a Hash, keys in
  snake_case or camelCase (the same policy file as the CLI's `--policy`, read
  with `JSON.parse`, works). The engine checks it there: a policy it does not
  take raises `XlsxGraft::PolicyInvalid`, whose `diagnostics` say what is wrong
  (for each key, a `policy_invalid` diagnostic).
- `verify(path, ...)` and `verify_bytes(data, ...)` take the rules of the
  call as keywords, each optional: `targets:`, `best_effort:`,
  `allow_functions:`, `allow_parts:` (Arrays of Strings) and `fail_on:`
  (`"warn"` to fail on a warning too).
- The result is successful when the engine's `ok` is true: no error
  diagnostic, and with `fail_on: "warn"` (the call's or the policy's) no
  warning either, so such a result can fail with no error diagnostic.
  `result.value` is always `nil`; `result.diagnostics` has what was found.
  `result.error` is set when the file cannot be read (`io`) or the call
  itself failed (`panic`, `null_arg`, `unknown_status`,
  `diagnostics_unreadable`). A fault of
  the rules is a `rules_invalid` or `target_invalid` diagnostic, and the
  workbook is not opened.
- A policy and rules that make more than 2 MiB of JSON, or a value JSON cannot
  write, or a policy that is not a Hash, raise `ArgumentError`; a path or data of the wrong class raises
  Ruby's own `TypeError` or `NoMethodError`. For a path that is not UTF-8, the
  gem reads the file itself: if it cannot, the result is the `io` error and
  the rules are not checked.
- A verifier does not change once it is made: share one between threads, and
  call it from them at once.

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
its block (`XlsxGraft::ClosedError`), a policy the engine does not take
(`XlsxGraft::PolicyInvalid`), and a call made wrongly
(`ArgumentError`: no block, rules too large or not JSON, a zone `Time` does not take, a sheet that is
neither a name nor an index).

## Where the engine library comes from

The gem never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library:` argument, as in `XlsxGraft.version(library: "/path/to/libxlsx_graft_capi.so")`
   or `XlsxReader.open(path, library: ...)`
3. the copy shipped inside the gem, in `lib/xlsx_graft/native/`

The gem for a platform ships the library, so `gem install xlsx_graft` needs no
engine of its own on Linux (glibc), macOS and Windows x64. On any other
platform RubyGems installs the gem with no engine, and loading it raises an
error that names the three ways above.

Ruby 3.4 or newer.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Ruby installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:ruby
```
