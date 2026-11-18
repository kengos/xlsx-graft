package xlsxgraft

// Grid is the cells of one sheet, the value of [Viewer.DescribeSheet]. Names
// and text are made for display (control characters removed, long text cut),
// so they are not exact; read the rows with [Open] for exact values.
type Grid struct {
	// Sheet is the sheet shown, nil when none was found.
	Sheet *SheetInfo
	// Dimension is the range the sheet declares, nil when it declares none.
	Dimension *string
	// Used is the range that holds cells, nil when the sheet is empty.
	Used *string
	// Shown is the range of the cells shown, nil when none are.
	Shown *string
	// Truncated says which limits cut what is shown.
	Truncated GridTruncated
	// Complete is whether the whole sheet was read.
	Complete bool
	// Rows are the rows shown.
	Rows []GridRow
	// Merges are the merged ranges (`A1:C1`).
	Merges []string
}

// GridTruncated says which of a grid's limits cut what it shows.
type GridTruncated struct {
	// Rows is whether rows were left out.
	Rows bool
	// Columns is whether columns were left out.
	Columns bool
	// Merges is whether merged ranges were left out.
	Merges bool
}

// GridRow is one row of a grid.
type GridRow struct {
	// Row is the row's number, from 1.
	Row int
	// Cells are the row's cells that hold a value, from the left.
	Cells []GridCell
}

// GridCell is one cell of a grid that holds a value.
type GridCell struct {
	// Ref is the cell's reference (`B3`).
	Ref string
	// Type is the cell's type: `string`, `number`, `boolean`, `date`, `time`,
	// `datetime`, `elapsed` or `error`.
	Type string
	// Value is the value of the cell's `value` key in the CLI's `--format json`
	// output: a string (also for a date, a time, a
	// date-time and an error), a bool, an int64 for a number written as an
	// integer that fits in an int64 (an elapsed time's milliseconds), or a
	// float64 for any other number (one written as an integer that does not
	// fit in an int64 included).
	Value any
	// Truncated is whether a long text was cut for display.
	Truncated bool
}
