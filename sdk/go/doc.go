// Package xlsxgraft is the Go binding for xlsx-graft, an engine that reads,
// verifies, creates and edits xlsx workbooks.
//
// The engine is its CLI, xlsx-graft, run as a subprocess; nothing here
// reimplements it. A call returns a [Result] rather than failing with an
// error: whether the call worked, the value when it did, and the diagnostics
// either way. The error beside it is a setup with no runnable CLI in it
// ([*CLINotFoundError]), and for [Open] the caller's own: the error its
// function returned, its context done, or a negative sheet index.
//
//	result, err := xlsxgraft.Open(ctx, "book.xlsx", func(book *xlsxgraft.Book) (int, error) {
//		rows := book.SheetAt(0).Rows()
//		defer rows.Close()
//		count := 0
//		for rows.Next() {
//			count++
//		}
//		return count, rows.Err()
//	})
//	if err != nil {
//		log.Fatal(err)
//	}
//	if result.Success {
//		fmt.Println(result.Value, "rows")
//	}
//
// A [Verifier] checks a workbook against the host's policy and the rules of
// the call, by running `xlsx-graft verify`:
//
//	verifier, err := xlsxgraft.NewVerifier(map[string]any{"maxEntries": 1000})
//	if err != nil {
//		log.Fatal(err)
//	}
//	result, err := verifier.Verify(ctx, "book.xlsx", xlsxgraft.VerifyRules{AllowParts: []string{"macro"}})
//	if err != nil {
//		log.Fatal(err)
//	}
//	fmt.Println(result.Success)
package xlsxgraft
