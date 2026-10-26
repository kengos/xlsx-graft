# xlsx-graft (PHP)

PHP bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an
engine that reads, verifies, creates and edits xlsx workbooks. The engine is
its CLI, `xlsx-graft`, which the package runs as a subprocess.

This is a skeleton: the only call so far is the engine's version.

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

Calls return a `Result` rather than throwing. The one thing that throws is a
CLI that is missing or cannot be run (`CliNotFoundException`), a PHP that
cannot start one included: `proc_open` disabled, or no temporary file for its
stderr.

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
