# xlsx-graft (Go)

Go bindings for [xlsx-graft](https://github.com/kengos/xlsx-graft), an engine
that reads, verifies, creates and edits xlsx workbooks. The engine is its CLI,
`xlsx-graft`, which the package runs as a subprocess.

This is a skeleton: the only call so far is the engine's version.

```go
import xlsxgraft "github.com/kengos/xlsx-graft/sdk/go"

result, err := xlsxgraft.Version()
if err != nil {
	log.Fatal(err)
}
if result.Success {
	fmt.Println(result.Value)
} else {
	for _, d := range result.Diagnostics {
		fmt.Fprintf(os.Stderr, "%s: %s\n", d.Code, d.Message)
	}
}
```

Calls return a `Result` rather than an error. The one error is a CLI that is
missing or cannot be started (`*CLINotFoundError`).

## Where the engine CLI comes from

The package never downloads anything. It runs the first of:

1. the path in `XLSX_GRAFT_CLI` (an empty value counts as unset)
2. the path passed, as in `xlsxgraft.Version(xlsxgraft.WithCLI("/path/to/xlsx-graft"))`
3. `xlsx-graft` (or `xlsx-graft.exe`) on `PATH`

A name with no directory in it, in `XLSX_GRAFT_CLI` or passed, is looked up
on `PATH` by `os/exec`. On Windows the CLI is an `.exe`: one with no
extension, set or passed, gets `.exe` added.

Go 1.26 or newer. No dependencies.

## Development

Everything runs in Docker through `make` at the repository root; you need no
Go installed. See [docs/sdk.md](../../docs/sdk.md).

```sh
make sdk:verify:go
```
