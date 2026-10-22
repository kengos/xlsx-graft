# xlsx_graft (Ruby)

Ruby bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is
a shared C library (`libxlsx_graft_capi`) loaded through fiddle.

This is a skeleton: the only call so far is the engine's version.

```ruby
require "xlsx_graft"

result = XlsxGraft.version
if result.success?
  puts result.value
else
  result.diagnostics.each { |d| warn "#{d.code}: #{d.message}" }
end
```

Calls return a `XlsxGraft::Result` rather than raising. The one thing that
raises is a missing engine library (`XlsxGraft::LibraryNotFound`).

## Where the engine library comes from

The gem never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library:` argument, as in `XlsxGraft.version(library: "/path/to/libxlsx_graft_capi.so")`
3. the copy shipped inside the gem, in `lib/xlsx_graft/native/`

Ruby 3.4 or newer.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Ruby installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:ruby
```
