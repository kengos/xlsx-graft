// Package xlsxgraft is the Go binding for xlsx-graft, an engine that reads,
// verifies, creates and edits xlsx workbooks.
//
// The engine is its CLI, xlsx-graft, run as a subprocess; nothing here
// reimplements it. A call returns a [Result] rather than failing with an
// error: whether the call worked, the value when it did, and the diagnostics
// either way. The error beside it is only ever a setup with no runnable CLI
// in it ([*CLINotFoundError]).
//
//	result, err := xlsxgraft.Version()
//	if err != nil {
//		log.Fatal(err)
//	}
//	if result.Success {
//		fmt.Println(result.Value)
//	}
package xlsxgraft
