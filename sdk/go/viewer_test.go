package xlsxgraft

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"sync"
	"testing"
)

// A Viewer against the real CLI (docs/sdk.md § Views in an SDK), on the
// workbooks of fixtures/.

const (
	merged       = "../../fixtures/merged.xlsx"
	externalLink = "../../fixtures/external-link.xlsx"
)

// engineViewer is a Viewer that runs the engine's CLI.
func engineViewer(t *testing.T) *Viewer {
	t.Helper()
	viewer, err := NewViewer(WithCLI(engineCLI(t)))
	if err != nil {
		t.Fatal(err)
	}
	return viewer
}

func viewTree(t *testing.T, path string) Result[*Tree] {
	t.Helper()
	result, err := engineViewer(t).Tree(context.Background(), path)
	if err != nil {
		t.Fatal(err)
	}
	return result
}

func viewGrid(t *testing.T, path string, request SheetRequest) Result[*Grid] {
	t.Helper()
	result, err := engineViewer(t).DescribeSheet(context.Background(), path, request)
	if err != nil {
		t.Fatal(err)
	}
	return result
}

func readFixture(t *testing.T, path string) []byte {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func ptr[T any](value T) *T {
	return &value
}

// cellsOfGrid are the cells of grid by their reference.
func cellsOfGrid(grid *Grid) map[string]GridCell {
	cells := map[string]GridCell{}
	for _, row := range grid.Rows {
		for _, cell := range row.Cells {
			cells[cell.Ref] = cell
		}
	}
	return cells
}

func TestTreeListsThePartsAndThePackagesRelationships(t *testing.T) {
	result := viewTree(t, table)

	if !result.Success || result.Error != nil || result.Value == nil || len(result.Diagnostics) != 0 {
		t.Fatalf("result = %+v", result)
	}
	tree := result.Value
	var names []string
	for _, part := range tree.Parts {
		names = append(names, part.Name)
	}
	pointsAtWorkbook := false
	for _, relationship := range tree.Relationships {
		pointsAtWorkbook = pointsAtWorkbook || (relationship.Target.Part != nil && *relationship.Target.Part == "/xl/workbook.xml")
	}
	if !tree.Complete || !tree.RelationshipsKnown || !pointsAtWorkbook || !containsString(names, "/xl/workbook.xml") {
		t.Errorf("tree = %+v", tree)
	}
}

func containsString(list []string, want string) bool {
	for _, item := range list {
		if item == want {
			return true
		}
	}
	return false
}

func TestTreeBytesGivesTheTreeOfTheDataAndTheFilesOwn(t *testing.T) {
	byPath := viewTree(t, table)

	byData, err := engineViewer(t).TreeBytes(context.Background(), readFixture(t, table))

	if err != nil || !byData.Success || !reflect.DeepEqual(byData, byPath) {
		t.Errorf("by data %+v (%v), by path %+v", byData, err, byPath)
	}
}

func TestATargetOutsideThePackageIsExternalAndHasNoPart(t *testing.T) {
	tree := viewTree(t, externalLink).Value

	found := false
	for _, part := range tree.Parts {
		for _, relationship := range part.Relationships {
			found = found || (relationship.Target.External != nil && relationship.Target.Part == nil)
		}
	}
	if !found {
		t.Errorf("no relationship with an external target in %+v", tree)
	}
}

func TestTreeOfAFileThatCannotBeOpenedFailsWithIoAndAnEmptyTree(t *testing.T) {
	result := viewTree(t, "/nowhere/book.xlsx")

	if result.Success || result.Error == nil || result.Error.Code != "io" || result.Value == nil || len(result.Value.Parts) != 0 {
		t.Errorf("result = %+v, error %+v", result, result.Error)
	}
}

func TestTreeOfDataThatIsNoWorkbookFailsWithDiagnosticsAndAnEmptyTree(t *testing.T) {
	result, err := engineViewer(t).TreeBytes(context.Background(), []byte("not a zip"))

	if err != nil || result.Success || result.Error != nil || len(result.Diagnostics) == 0 || result.Value == nil || len(result.Value.Parts) != 0 {
		t.Errorf("result = %+v (%v)", result, err)
	}
}

func TestDescribeSheetShowsTheCellsOfTheFirstSheet(t *testing.T) {
	result := viewGrid(t, merged, SheetRequest{})

	if !result.Success || result.Error != nil || result.Value == nil {
		t.Fatalf("result = %+v", result)
	}
	grid := result.Value
	cells := cellsOfGrid(grid)
	wantSheet := SheetInfo{Index: 0, Name: "Sheet1", Kind: Worksheet, State: Visible}
	if !reflect.DeepEqual(grid.Sheet, &wantSheet) {
		t.Errorf("sheet = %+v", grid.Sheet)
	}
	for name, got := range map[string]*string{"dimension": grid.Dimension, "used": grid.Used, "shown": grid.Shown} {
		if got == nil || *got != "A1:E8" {
			t.Errorf("%s = %v", name, got)
		}
	}
	if grid.Truncated != (GridTruncated{}) || !grid.Complete || !reflect.DeepEqual(grid.Merges, []string{"A1:C1", "A3:A5"}) {
		t.Errorf("truncated %+v, complete %v, merges %v", grid.Truncated, grid.Complete, grid.Merges)
	}
	for ref, want := range map[string]GridCell{
		"A1": {Ref: "A1", Type: "string", Value: "Quarterly report"},
		"C3": {Ref: "C3", Type: "number", Value: 1.5},
		"C4": {Ref: "C4", Type: "number", Value: 2.0},
		"C6": {Ref: "C6", Type: "date", Value: "2024-01-15"},
	} {
		if !reflect.DeepEqual(cells[ref], want) {
			t.Errorf("%s = %+v, want %+v", ref, cells[ref], want)
		}
	}
}

func TestARangeAndALimitNarrowWhatIsShown(t *testing.T) {
	ranged := viewGrid(t, merged, SheetRequest{Range: "A1:C3"}).Value
	limited := viewGrid(t, merged, SheetRequest{MaxRows: 2}).Value

	if ranged.Shown == nil || *ranged.Shown != "A1:C3" || limited.Shown == nil || *limited.Shown != "A1:E2" || !limited.Truncated.Rows {
		t.Errorf("ranged %+v, limited %+v", ranged, limited)
	}
}

func TestASheetIndexOfZeroIsTheSameAsNoSheet(t *testing.T) {
	first := viewGrid(t, merged, SheetRequest{})

	byIndex := viewGrid(t, merged, SheetRequest{SheetIndex: ptr(0)})
	byName := viewGrid(t, merged, SheetRequest{Sheet: "Sheet1"})

	if !reflect.DeepEqual(byIndex, first) || !reflect.DeepEqual(byName, first) {
		t.Errorf("first %+v, by index %+v, by name %+v", first, byIndex, byName)
	}
}

func TestDescribeSheetBytesGivesTheGridOfTheDataAndTheFilesOwn(t *testing.T) {
	byPath := viewGrid(t, merged, SheetRequest{MaxRows: 3})

	byData, err := engineViewer(t).DescribeSheetBytes(context.Background(), readFixture(t, merged), SheetRequest{MaxRows: 3})

	if err != nil || !byData.Success || !reflect.DeepEqual(byData, byPath) {
		t.Errorf("by data %+v (%v), by path %+v", byData, err, byPath)
	}
}

func TestASheetThatIsNotFoundFailsWithAGridHoldingNoSheet(t *testing.T) {
	result := viewGrid(t, table, SheetRequest{Sheet: "Nope"})

	if result.Success || result.Error == nil || result.Error.Code != "sheet_not_found" || result.Value == nil || result.Value.Sheet != nil {
		t.Errorf("result = %+v, error %+v", result, result.Error)
	}
}

func TestARequestTheCLIRefusesIsAnInvalidRequestErrorInTheCLIsWords(t *testing.T) {
	viewer := engineViewer(t)
	for name, tc := range map[string]struct {
		request SheetRequest
		option  string // a word of the CLI's reason
	}{
		"a negative row limit":      {SheetRequest{MaxRows: -1}, "--max-rows"},
		"a range that is not A1":    {SheetRequest{Range: "nope"}, "--range"},
		"a sheet name and an index": {SheetRequest{Sheet: "Sheet1", SheetIndex: ptr(0)}, "--sheet"},
		"a negative column limit":   {SheetRequest{MaxColumns: -1}, "--max-columns"},
		"a negative sheet index":    {SheetRequest{SheetIndex: ptr(-1)}, "--sheet-index"},
		"a range that is not UTF-8": {SheetRequest{Range: "\xff"}, "invalid UTF-8"},
	} {
		result, err := viewer.DescribeSheet(context.Background(), merged, tc.request)
		byData, errData := viewer.DescribeSheetBytes(context.Background(), readFixture(t, merged), tc.request)

		var refused, refusedData *InvalidRequestError
		if !errors.As(err, &refused) || !errors.As(errData, &refusedData) {
			t.Errorf("%s: errors %v, %v", name, err, errData)
			continue
		}
		if !strings.Contains(refused.Message, tc.option) || strings.HasPrefix(refused.Message, "xlsx-graft:") ||
			refused.Error() != refused.Message || refusedData.Message != refused.Message ||
			!reflect.DeepEqual(result, Result[*Grid]{}) || !reflect.DeepEqual(byData, Result[*Grid]{}) {
			t.Errorf("%s: message %q, results %+v, %+v", name, refused.Message, result, byData)
		}
	}
}

func TestOneViewerServesGoroutinesThatCallItAtOnce(t *testing.T) {
	viewer := engineViewer(t)
	data := readFixture(t, merged)
	grids := make([]Result[*Grid], 4)
	trees := make([]Result[*Tree], 4)
	var wg sync.WaitGroup
	for i := range grids {
		wg.Add(1)
		go func() {
			defer wg.Done()
			var err error
			grids[i], err = viewer.DescribeSheetBytes(context.Background(), data, SheetRequest{})
			if err != nil {
				t.Error(err)
			}
			trees[i], err = viewer.TreeBytes(context.Background(), data)
			if err != nil {
				t.Error(err)
			}
		}()
	}
	wg.Wait()

	for i := range grids {
		if !grids[i].Success || !trees[i].Success || !reflect.DeepEqual(grids[i], grids[0]) || !reflect.DeepEqual(trees[i], trees[0]) {
			t.Errorf("goroutine %d: grid %+v, tree %+v", i, grids[i], trees[i])
		}
	}
}

func TestNoTemporaryFileIsLeftBehindByAView(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("TMPDIR", dir)
	viewer := engineViewer(t)
	data := readFixture(t, table)

	tree, err := viewer.TreeBytes(context.Background(), data)
	grid, errGrid := viewer.DescribeSheetBytes(context.Background(), data, SheetRequest{})

	left, _ := filepath.Glob(filepath.Join(dir, "xlsx-graft-*"))
	if err != nil || errGrid != nil || !tree.Success || !grid.Success || len(left) != 0 {
		t.Errorf("tree %+v (%v), grid %+v (%v), left behind %v", tree, err, grid, errGrid, left)
	}
}

func TestNoCLIFoundIsACLINotFoundErrorOfNewViewer(t *testing.T) {
	t.Setenv(envVar, "")
	t.Setenv("PATH", t.TempDir())

	viewer, err := NewViewer()

	var notFound *CLINotFoundError
	if viewer != nil || !errors.As(err, &notFound) {
		t.Errorf("viewer %v, error %v", viewer, err)
	}
}
