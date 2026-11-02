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
	"time"
)

// Open against the real CLI: each value, the sheets, a sheet asked for,
// refused or stopped, and the book's scope.

// everyRow reads rows to their end.
func everyRow(rows *Rows) ([]Row, error) {
	defer rows.Close()
	var read []Row
	for rows.Next() {
		read = append(read, rows.Row())
	}
	return read, rows.Err()
}

// firstSheet reads the first sheet's rows of the workbook at path.
func firstSheet(t *testing.T, path string, opts ...Option) Result[[]Row] {
	t.Helper()
	result, err := Open(context.Background(), path, func(book *Book) ([]Row, error) {
		return everyRow(book.SheetAt(0).Rows())
	}, opts...)
	if err != nil {
		t.Fatal(err)
	}
	return result
}

// A row of each kind of value, with an empty column and a row left out: a
// shared string, an inline string, then a number, a boolean, an error value,
// a formula's string and an ISO 8601 date-time, then a date, a time, a
// date-time and elapsed time made of numbers by their formats.
const eachValue = `<row r="1"><c r="A1" t="s"><v>0</v></c><c r="C1" t="inlineStr"><is><t>in&#9;line</t></is></c></row>` +
	`<row r="3"><c r="A3"><v>3</v></c><c r="B3" t="b"><v>1</v></c><c r="C3" t="e"><v>#N/A</v></c>` +
	`<c r="D3" t="str"><f>A3</f><v>formula</v></c><c r="E3" t="d"><v>2024-01-31T08:30:00</v></c></row>` +
	`<row r="4"><c r="A4" s="1"><v>45322</v></c><c r="B4" s="2"><v>0.5</v></c>` +
	`<c r="C4" s="3"><v>45322.25</v></c><c r="D4" s="4"><v>1.5</v></c></row>`

func TestReadsEachValueAsItsType(t *testing.T) {
	engineCLI(t)
	got := firstSheet(t, writeWorkbook(t, worksheet("Data", eachValue)))

	want := ok([]Row{
		{1, []any{"a", nil, "in\tline"}},
		{3, []any{3.0, true, CellError{"#N/A"}, "formula", time.Date(2024, 1, 31, 8, 30, 0, 0, time.UTC)}},
		{4, []any{Date{2024, time.January, 31}, TimeOfDay{12, 0, 0, 0}, time.Date(2024, 1, 31, 6, 0, 0, 0, time.UTC), 36 * time.Hour}},
	})
	if !reflect.DeepEqual(got, want) {
		t.Errorf("result = %+v, want %+v", got, want)
	}
}

func TestPlacesADateTimeInTheLocationGiven(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", `<row><c t="d"><v>2024-01-31T08:30:00.25</v></c></row>`))
	tokyo, err := time.LoadLocation("Asia/Tokyo")
	if err != nil {
		t.Fatal(err)
	}

	for _, loc := range []*time.Location{tokyo, time.FixedZone("+09:00", 9*60*60), nil} {
		got := firstSheet(t, path, WithLocation(loc)).Value[0].Values[0].(time.Time)
		in := loc
		if loc == nil {
			in = time.UTC
		}
		if want := time.Date(2024, 1, 31, 8, 30, 0, 250*int(time.Millisecond), in); !got.Equal(want) || got.Location() != in {
			t.Errorf("%v: got %v, want %v", loc, got, want)
		}
	}
}

func TestADateTimeTheLocationSkipsOrHasTwiceStopsItsSheet(t *testing.T) {
	engineCLI(t)
	newYork, err := time.LoadLocation("America/New_York")
	if err != nil {
		t.Fatal(err)
	}
	path := writeWorkbook(t,
		worksheet("Skipped", `<row r="1"><c t="d"><v>2024-03-10T01:59:00</v></c></row><row r="2"><c t="d"><v>2024-03-10T02:30:00</v></c></row>`),
		worksheet("Repeated", `<row r="5"><c t="d"><v>2024-11-03T01:30:00</v></c></row>`),
	)

	for sheet, want := range map[string]string{
		"Skipped":  "row 2: 2024-03-10 02:30:00.000 cannot be placed in America/New_York: its daylight saving skips it",
		"Repeated": "row 5: 2024-11-03 01:30:00.000 cannot be placed in America/New_York: it has it twice",
	} {
		got, err := Open(context.Background(), path, func(book *Book) ([]Row, error) {
			return everyRow(book.Sheet(sheet).Rows())
		}, WithLocation(newYork))
		if err != nil || got.Success || got.Error == nil || !reflect.DeepEqual(*got.Error, problem("datetime_unrepresentable", want)) || got.Diagnostics != nil {
			t.Errorf("%s: result = %+v, %v", sheet, got, err)
		}
	}
}

func TestListsTheSheetsAndTheirProblems(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", `<row><c t="s"><v>1</v></c></row>`), chartsheet("Chart", "veryHidden"), unrelated("Lost", "hidden"))
	var listed [][]SheetInfo
	got, err := Open(context.Background(), path, func(book *Book) (any, error) {
		first, err := book.Sheets()
		listed = append(listed, first)
		if _, err := everyRow(book.Sheet("data").Rows()); err != nil {
			return nil, err
		}
		again, _ := book.Sheets()
		listed = append(listed, again)
		return nil, err
	})

	sheets := []SheetInfo{{0, "Data", Worksheet, Visible}, {1, "Chart", Chartsheet, VeryHidden}, {2, "Lost", SheetKindUnknown, Hidden}}
	if err != nil || !reflect.DeepEqual(listed, [][]SheetInfo{sheets, sheets}) {
		t.Errorf("listed %+v, %v", listed, err)
	}
	// The part not known is the workbook's error: the read fails with no
	// error, its problem found by both runs and kept once.
	if got.Success || got.Error != nil || len(got.Diagnostics) != 1 {
		t.Fatalf("result = %+v", got)
	}
	if d := got.Diagnostics[0]; d.Code != "sheet_relationship_missing" || d.Severity != "error" || len(d.Location) == 0 {
		t.Errorf("diagnostic = %+v", d)
	}
}

func TestChoosesASheetByNameLetterCaseAsideOrByIndex(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", `<row r="1"/>`), worksheet("-Other", `<row r="2"/>`))
	numbers := func(rows *Rows) []int {
		read, _ := everyRow(rows)
		var numbers []int
		for _, row := range read {
			numbers = append(numbers, row.Number)
		}
		return numbers
	}

	got, err := Open(context.Background(), path, func(book *Book) ([][]int, error) {
		return [][]int{numbers(book.SheetAt(0).Rows()), numbers(book.Sheet("-oTHER").Rows()), numbers(book.SheetAt(1).Rows())}, nil
	})
	if want := ok([][]int{{1}, {2}, {2}}); err != nil || !reflect.DeepEqual(got, want) {
		t.Errorf("result = %+v, %v; want %+v", got, err, want)
	}
}

func TestABookIsSharedByGoroutines(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("A", `<row r="1"/><row r="2"/>`), worksheet("B", `<row r="3"/>`))
	got, err := Open(context.Background(), path, func(book *Book) ([][]Row, error) {
		read := make([][]Row, 4)
		errs := make([]error, 4)
		var wg sync.WaitGroup
		for i, sheet := range []*Sheet{book.Sheet("A"), book.Sheet("B"), book.SheetAt(0), book.SheetAt(1)} {
			wg.Go(func() { read[i], errs[i] = everyRow(sheet.Rows()) })
		}
		for range 2 {
			wg.Go(func() {
				if sheets, err := book.Sheets(); err != nil || len(sheets) != 2 {
					t.Errorf("Sheets = %+v, %v", sheets, err)
				}
			})
		}
		wg.Wait()
		return read, errors.Join(errs...)
	})

	a, b := []Row{{1, []any{}}, {2, []any{}}}, []Row{{3, []any{}}}
	if want := ok([][]Row{a, b, a, b}); err != nil || !reflect.DeepEqual(got, want) {
		t.Errorf("result = %+v, %v; want %+v", got, err, want)
	}
}

func TestARefusedSheetReadsNothingAndStaysTheError(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", `<row/>`), chartsheet("Chart", ""))
	got, err := Open(context.Background(), path, func(book *Book) (int, error) {
		count := 0
		for _, sheet := range []*Sheet{book.Sheet("none"), book.SheetAt(5), book.Sheet("Chart"), book.SheetAt(0)} {
			read, err := everyRow(sheet.Rows())
			if err != nil {
				return 0, err
			}
			count += len(read)
		}
		return count, nil
	})

	if want := refused[int](problem("sheet_not_found", `no sheet is named "none"`)); err != nil || !reflect.DeepEqual(got, want) {
		t.Errorf("result = %+v, %v; want %+v", got, err, want)
	}
}

func TestAProblemInASheetEndsItsRowsAfterTheRowsBefore(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", `<row r="1"><c t="s"><v>1</v></c></row><row r="2"><c t="s"><v>9</v></c></row><row r="3"/>`))
	var read []Row
	got, err := Open(context.Background(), path, func(book *Book) (any, error) {
		var err error
		read, err = everyRow(book.SheetAt(0).Rows())
		return nil, err
	})

	if err != nil || !reflect.DeepEqual(read, []Row{{1, []any{"b"}}}) || got.Success || got.Error != nil {
		t.Fatalf("read %+v, result %+v, %v", read, got, err)
	}
	d := got.Diagnostics[0]
	if d.Code != "shared_string_missing" || !reflect.DeepEqual(d.Args, map[string]any{"index": 9.0}) || !reflect.DeepEqual(d.Location, map[string]any{"part": "/xl/sheets/sheet1.xml"}) {
		t.Errorf("diagnostic = %+v", d)
	}
}

func TestStoppingEarlyIsNoFailure(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", strings.Repeat(`<row><c t="inlineStr"><is><t>a long enough value</t></is></c></row>`, 50000)))
	var left *Rows
	got, err := Open(context.Background(), path, func(book *Book) (int, error) {
		rows := book.SheetAt(0).Rows()
		rows.Next()
		rows.Close()
		rows.Close()
		if rows.Next() {
			t.Error("Next after Close read a row")
		}
		// Rows left standing stop when the book closes.
		left = book.SheetAt(0).Rows()
		left.Next()
		return rows.Row().Number, rows.Err()
	})

	if want := ok(1); err != nil || !reflect.DeepEqual(got, want) {
		t.Errorf("result = %+v, %v; want %+v", got, err, want)
	}
	if left.Next() || !errors.Is(left.Err(), ErrClosed) {
		t.Errorf("rows after the close: Err = %v, want ErrClosed", left.Err())
	}
}

func TestAFileThatIsNoWorkbookListsNoSheet(t *testing.T) {
	engineCLI(t)
	path := filepath.Join(t.TempDir(), "book.xlsx")
	if err := os.WriteFile(path, []byte("not a zip"), 0o600); err != nil {
		t.Fatal(err)
	}
	got, err := Open(context.Background(), path, func(book *Book) ([]SheetInfo, error) {
		sheets, err := book.Sheets()
		if _, err := everyRow(book.SheetAt(0).Rows()); err != nil {
			return nil, err
		}
		return sheets, err
	})

	if err != nil || got.Error == nil || got.Error.Code != "workbook_unreadable" || len(got.Diagnostics) != 1 {
		t.Errorf("result = %+v, %v", got, err)
	}
}

func TestAFileThatWillNotOpenFailsTheCLIRun(t *testing.T) {
	engineCLI(t)
	path := filepath.Join(t.TempDir(), "missing.xlsx")
	got, err := Open(context.Background(), path, func(book *Book) ([]SheetInfo, error) {
		return book.Sheets()
	})

	if err != nil || got.Error == nil || got.Error.Code != "exit_status" ||
		!strings.HasPrefix(got.Error.Message, "the CLI exited with status 1: xlsx-graft: failed to open "+path+": ") {
		t.Errorf("result = %+v, %v", got, err)
	}
}

func TestAPathWithANulIsRefusedBeforeRead(t *testing.T) {
	engineCLI(t)
	got, err := Open(context.Background(), "book\x00.xlsx", func(*Book) (int, error) {
		t.Error("read ran")
		return 0, nil
	})

	if want := refused[int](problem("invalid_arg", "a path holds no NUL")); err != nil || !reflect.DeepEqual(got, want) {
		t.Errorf("result = %+v, %v; want %+v", got, err, want)
	}
}

func TestASheetNameThatIsNotUTF8OrHoldsANulIsRefused(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", ""))
	got, err := Open(context.Background(), path, func(book *Book) (int, error) {
		bad, err := everyRow(book.Sheet("\xff").Rows())
		nul, _ := everyRow(book.Sheet("a\x00").Rows())
		return len(bad) + len(nul), err
	})

	if want := refused[int](problem("invalid_arg", "a sheet name is UTF-8 text with no NUL")); err != nil || !reflect.DeepEqual(got, want) {
		t.Errorf("result = %+v, %v; want %+v", got, err, want)
	}
}

func TestReadsBytesFromAFileOfTheBooksOwn(t *testing.T) {
	engineCLI(t)
	data, err := os.ReadFile(writeWorkbook(t, worksheet("Data", `<row r="7"/>`)))
	if err != nil {
		t.Fatal(err)
	}
	dir := t.TempDir()
	t.Setenv("TMPDIR", dir)
	var during []string
	got, err := OpenBytes(context.Background(), data, func(book *Book) ([]Row, error) {
		during, _ = filepath.Glob(filepath.Join(dir, "xlsx-graft-*.xlsx"))
		return everyRow(book.SheetAt(0).Rows())
	})

	if want := ok([]Row{{7, []any{}}}); err != nil || !reflect.DeepEqual(got, want) {
		t.Errorf("result = %+v, %v; want %+v", got, err, want)
	}
	if len(during) != 1 {
		t.Fatalf("files during the read: %v", during)
	}
	if after, _ := filepath.Glob(filepath.Join(dir, "*")); len(after) != 0 {
		t.Errorf("files after the close: %v", after)
	}
}

func TestBytesGoToAFileOnlyTheUserMayRead(t *testing.T) {
	dir := t.TempDir()
	path, err := writeTemporary(dir, []byte("bytes"))
	if err != nil {
		t.Fatal(err)
	}
	info, err := os.Stat(path)
	data, _ := os.ReadFile(path)
	if err != nil || info.Mode().Perm() != 0o600 || string(data) != "bytes" || filepath.Dir(path) != dir {
		t.Errorf("%s: %v, %q, %v", path, info.Mode(), data, err)
	}
}

func TestBytesThatCannotBeWrittenAreIo(t *testing.T) {
	engineCLI(t)
	t.Setenv("TMPDIR", filepath.Join(t.TempDir(), "none"))
	got, err := OpenBytes(context.Background(), []byte("x"), func(*Book) (int, error) {
		t.Error("read ran")
		return 0, nil
	})

	if err != nil || got.Error == nil || got.Error.Code != "io" ||
		!strings.HasPrefix(got.Error.Message, "the workbook could not be written to a temporary file (open ") {
		t.Errorf("result = %+v, %v", got, err)
	}
}

func TestReadsErrorIsReturnedTheBookClosedFirst(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", `<row/><row/>`))
	mine := errors.New("the caller")
	var kept *Book
	got, err := Open(context.Background(), path, func(book *Book) (int, error) {
		kept = book
		book.SheetAt(0).Rows().Next()
		return 1, mine
	})

	if err != mine || !reflect.DeepEqual(got, Result[int]{}) {
		t.Errorf("result = %+v, %v", got, err)
	}
	if _, err := kept.Sheets(); err != ErrClosed {
		t.Errorf("Sheets after the close: %v", err)
	}
}

func TestAPanicInReadPropagatesTheBookClosedFirst(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", `<row/>`))
	var kept *Book
	defer func() {
		if recovered := recover(); recovered != "the caller" || kept.check() != ErrClosed {
			t.Errorf("recovered %v", recovered)
		}
	}()
	_, _ = Open(context.Background(), path, func(book *Book) (int, error) {
		kept = book
		panic("the caller")
	})
}

func TestANegativeIndexIsAnError(t *testing.T) {
	engineCLI(t)
	path := writeWorkbook(t, worksheet("Data", ""))
	got, err := Open(context.Background(), path, func(book *Book) (int, error) {
		rows := book.SheetAt(-1).Rows()
		// Ignored: Open returns it all the same.
		rows.Next()
		return 0, nil
	})

	if err == nil || err.Error() != "xlsxgraft: a sheet index is from 0, not -1" || !reflect.DeepEqual(got, Result[int]{}) {
		t.Errorf("result = %+v, %v", got, err)
	}
}
