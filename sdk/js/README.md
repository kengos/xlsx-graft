# xlsx-graft (Node.js)

Node.js bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is a
Node.js addon (`xlsx_graft.node`) that links its C ABI in.

This is a skeleton: the only call so far is the engine's version.

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

Calls return a `Result` rather than throwing. The one thing that throws is a
missing engine addon (`LibraryNotFoundError`).

The package is ESM; `require('xlsx-graft')` loads it as well, through
Node.js's `require()` of ES modules.

## Where the engine addon comes from

The package never downloads anything. It loads the first of:

1. the path in `XLSX_GRAFT_ADDON` (an empty value counts as unset)
2. the `addon` option, as in `version({ addon: '/path/to/xlsx_graft.node' })`
3. the copy shipped inside the package, at `native/xlsx_graft.node`

Node.js 24 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Node.js installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:js
```
