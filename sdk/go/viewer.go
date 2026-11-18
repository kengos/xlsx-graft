package xlsxgraft

import (
	"context"
	"os"
	"strconv"
)

// Viewer shows the parts of a workbook and the cells of its sheets, by running
// `xlsx-graft tree` and `xlsx-graft describe-sheet` (docs/sdk.md § Views in an
// SDK). A view takes no policy: the limits a workbook is read under are the
// CLI's defaults.
//
// A Viewer holds the CLI's path, which does not change after [NewViewer]
// returns, so one Viewer may be used by any number of goroutines at once. Each
// call writes the temporary file it needs under a name of its own.
//
//	viewer, err := xlsxgraft.NewViewer()
//	if err != nil {
//		return err
//	}
//	tree, err := viewer.Tree(ctx, "book.xlsx")
//	if err != nil {
//		return err
//	}
//	if tree.Value != nil {
//		for _, part := range tree.Value.Parts {
//			fmt.Println(part.Name)
//		}
//	}
//
//	grid, err := viewer.DescribeSheet(ctx, "book.xlsx", xlsxgraft.SheetRequest{
//		Sheet:   "Data",
//		Range:   "A1:H50",
//		MaxRows: 20,
//	})
//	if err != nil {
//		return err
//	}
//	if grid.Value != nil {
//		for _, row := range grid.Value.Rows {
//			for _, cell := range row.Cells {
//				fmt.Println(cell.Ref, cell.Value)
//			}
//		}
//	}
//
// The result of a view is a [Result] of a [*Tree] or a [*Grid]: it succeeds
// when the engine's `ok` is true, and its Value is there also when it is not
// (a workbook that is broken, or a file that cannot be opened, gives a Tree of
// the parts that were read, perhaps none; a sheet that is not found gives a
// Grid whose Sheet is nil). Value is nil only when this package did not get one:
// Error is `exit_status`, `invalid_output`, `invalid_arg`, or `io` for a
// temporary file that could not be written. The zero Result returned beside a
// non-nil error also has a nil Value. A file that cannot be opened is an `io`
// Error, with a Value.
type Viewer struct {
	cli string
}

// SheetRequest says which sheet [Viewer.DescribeSheet] describes and how much
// of it. Every field is optional, and the zero value of a field (an empty
// string, a nil pointer, 0) is not passed to the CLI: `MaxRows: 0` is the
// CLI's default of 100 rows, not zero rows. This package checks a value
// only for a NUL (`invalid_arg`); the CLI judges the rest, and a value it
// refuses is an [*InvalidRequestError].
type SheetRequest struct {
	// Sheet is the sheet's name; without a sheet or a sheet index, the first
	// sheet (index 0) is described, whatever its kind.
	Sheet string
	// SheetIndex is the sheet's index, from 0. Giving it with Sheet is refused
	// by the CLI.
	SheetIndex *int
	// Range is the window of cells, an A1 range (`A1:H50`).
	Range string
	// MaxRows is the most rows to show, 1 to 10000; the CLI's default of 100
	// when not given.
	MaxRows int
	// MaxColumns is the most columns to show, 1 to 1000; 30 when not given.
	MaxColumns int
}

// args are the request as arguments of the CLI, each value in one argument
// with its flag (`--sheet=-x`), so that a value starting with `-` is not read
// as a flag.
func (r SheetRequest) args() []string {
	var args []string
	if r.Sheet != "" {
		args = append(args, "--sheet="+r.Sheet)
	}
	if r.SheetIndex != nil {
		args = append(args, "--sheet-index="+strconv.Itoa(*r.SheetIndex))
	}
	if r.Range != "" {
		args = append(args, "--range="+r.Range)
	}
	if r.MaxRows != 0 {
		args = append(args, "--max-rows="+strconv.Itoa(r.MaxRows))
	}
	if r.MaxColumns != 0 {
		args = append(args, "--max-columns="+strconv.Itoa(r.MaxColumns))
	}
	return args
}

// NewViewer makes a Viewer. The error is a [*CLINotFoundError] when no CLI is
// found (the CLI is looked up as [Open] looks it up; [WithLocation] has no
// effect).
func NewViewer(opts ...Option) (*Viewer, error) {
	cli, err := findCLI(apply(opts).cli, os.Getenv)
	if err != nil {
		return nil, err
	}
	return &Viewer{cli: cli}, nil
}

// Tree lists the parts of the workbook at path and the relationships between
// them.
//
// The result is a success when the engine's `ok` is true. A file that cannot
// be opened is a failure whose Error is `io`; what is wrong inside the file
// (not a zip, a part that cannot be read) is a diagnostic, and the Value holds
// the parts that were read, none when the package could not be opened. A path
// holding a NUL is `invalid_arg`, and the CLI is not run. A CLI that gave no
// result is a failure of `exit_status` or `invalid_output`; exit status 2,
// which `tree` gives only for a command line this package built wrong, is
// always `exit_status`.
//
// The error is a [*CLINotFoundError] when the CLI cannot be started, or ctx's
// error when ctx ended the run (killing the CLI); the result is then the zero
// Result. A path that is not UTF-8 is given to the CLI as it is.
func (v *Viewer) Tree(ctx context.Context, path string) (Result[*Tree], error) {
	return viewPath(ctx, v.cli, treeView, nil, path)
}

// TreeBytes is as [Viewer.Tree], the workbook being data, written to a
// temporary file of the call's own (the CLI reads a path) that only its user
// may read, removed when the call returns. Changing data once the SDK has
// written the file has no effect, even while the call runs. A file that
// cannot be written is `io`, with no Value.
func (v *Viewer) TreeBytes(ctx context.Context, data []byte) (Result[*Tree], error) {
	return viewData(ctx, v.cli, treeView, nil, data)
}

// DescribeSheet shows the cells of one sheet of the workbook at path, within
// the window request gives.
//
// The result is as [Viewer.Tree]'s, with a [*Grid]: a sheet that cannot be
// chosen or read (`sheet_not_found`, `sheet_ambiguous`, `sheet_not_worksheet`,
// `sheet_unreadable`) is a failure with a Value. A path, a sheet name or a
// range holding a NUL is `invalid_arg`, and the CLI is not run.
//
// The error is as [Viewer.Tree]'s, and also an [*InvalidRequestError] when the
// CLI refused an argument of request (exit status 2).
func (v *Viewer) DescribeSheet(ctx context.Context, path string, request SheetRequest) (Result[*Grid], error) {
	return viewPath(ctx, v.cli, gridView, request.args(), path)
}

// DescribeSheetBytes is as [Viewer.DescribeSheet], the workbook being data,
// written to a temporary file as [Viewer.TreeBytes] writes it.
func (v *Viewer) DescribeSheetBytes(ctx context.Context, data []byte, request SheetRequest) (Result[*Grid], error) {
	return viewData(ctx, v.cli, gridView, request.args(), data)
}
