# XlsxGraft (.NET)

.NET bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is a
shared C library (`libxlsx_graft_capi`) called through unmanaged function
pointers.

This is a skeleton: the only call so far is the engine's version.

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

Calls return a `XlsxGraft.Result<T>` rather than throwing. The one thing that
throws is a missing engine library (`XlsxGraft.LibraryNotFoundException`).

## Where the engine library comes from

The package never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library` argument, as in `Engine.Version(library: "/path/to/libxlsx_graft_capi.so")`
3. a library in `native/` beside the application (no package ships one yet)

.NET 10 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
.NET installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:dotnet
```
