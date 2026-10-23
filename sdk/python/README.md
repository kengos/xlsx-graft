# xlsx-graft (Python)

Python bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is
a shared C library (`libxlsx_graft_capi`) loaded through ctypes.

This is a skeleton: the only call so far is the engine's version.

```python
import sys

import xlsx_graft

result = xlsx_graft.version()
if result.success:
    print(result.value)
else:
    for d in result.diagnostics:
        print(f"{d.code}: {d.message}", file=sys.stderr)
```

Calls return a `xlsx_graft.Result` rather than raising. The one thing that
raises is a missing engine library (`xlsx_graft.LibraryNotFoundError`).

## Where the engine library comes from

The package never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_LIBRARY` (an empty value counts as unset)
2. the `library` argument, as in `xlsx_graft.version(library="/path/to/libxlsx_graft_capi.so")`
3. the copy shipped inside the wheel, in `xlsx_graft/native/`

Python 3.11 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Python installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:python
```
