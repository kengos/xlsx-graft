package xlsxgraft

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"slices"
	"strings"
	"sync"
	"testing"
	"time"
)

// What a book makes of the runs the real CLI never gives (docs/sdk.md §
// Listing sheets and § Reading rows), from fake CLIs printing them.

const endLine = `{"ok":true,"sheet":{"index":0,"name":"Data"},"rows":1,"error":null,"diagnostics":[]}`

// lines is a shell command per line, each printing it.
func lines(printed ...string) string {
	var script strings.Builder
	for _, line := range printed {
		script.WriteString("printf '%s\\n' '" + strings.ReplaceAll(line, "'", `'\''`) + "'\n")
	}
	return script.String()
}

// readFake reads the first sheet's rows from a fake CLI whose `read` runs
// body, with the rows that came.
func readFake(t *testing.T, body string, opts ...Option) (Result[any], []Row) {
	t.Helper()
	t.Setenv(envVar, "")
	cli := fake(t, "[ \"$1\" = read ] || exit 99\n"+body)
	var read []Row
	result, err := Open(context.Background(), "book.xlsx", func(book *Book) (any, error) {
		var err error
		read, err = everyRow(book.SheetAt(0).Rows())
		return nil, err
	}, append(opts, WithCLI(cli))...)
	if err != nil {
		t.Fatal(err)
	}
	return result, read
}

// listFake is the sheets of a fake CLI whose `sheets` runs body.
func listFake(t *testing.T, body string) Result[[]SheetInfo] {
	t.Helper()
	t.Setenv(envVar, "")
	cli := fake(t, "[ \"$1\" = sheets ] || exit 99\n"+body)
	result, err := Open(context.Background(), "book.xlsx", func(book *Book) ([]SheetInfo, error) {
		return book.Sheets()
	}, WithCLI(cli))
	if err != nil {
		t.Fatal(err)
	}
	return result
}

func TestAnEndLineIsTheResult(t *testing.T) {
	diagnostic := `{"code":"x_y","severity":"warning","message":"m","args":{"n":1},"location":{"part":"/p"},"decidedBy":{"rule":{"id":"r"}},"fix":null,"origin":"o"}`
	end := `{"ok":false,"sheet":null,"rows":0,"error":{"code":"sheet_not_found","message":"no sheet"},"diagnostics":[` + diagnostic + `]}`
	got, read := readFake(t, lines(`{"row":2,"cells":[{"type":"number","value":3}]}`, end)+"exit 1")

	refusal := problem("sheet_not_found", "no sheet")
	want := Result[any]{Error: &refusal, Diagnostics: []Diagnostic{{
		Code: "x_y", Message: "m", Severity: "warning", Args: map[string]any{"n": 1.0},
		Location: map[string]any{"part": "/p"}, DecidedBy: map[string]any{"rule": map[string]any{"id": "r"}},
	}}}
	if !reflect.DeepEqual(got, want) || !reflect.DeepEqual(read, []Row{{2, []any{3.0}}}) {
		t.Errorf("result = %+v, rows %+v", got, read)
	}
}

func TestAStatusOtherThan0Or1OrASignalIsExitStatus(t *testing.T) {
	for body, want := range map[string]string{
		lines(`{"row":1,"cells":[]}`, endLine) + `echo "xlsx-graft: broken" >&2; exit 3`: "the CLI exited with status 3: xlsx-graft: broken",
		lines(`{"row":1,"cells":[]}`) + "kill -9 $$":                                     "the CLI was killed by signal 9",
		// A signal after the end line is still a failure.
		lines(endLine) + "kill -9 $$": "the CLI was killed by signal 9",
		// Exit 1 with no end line is the host failing.
		`echo "xlsx-graft: failed to read" >&2; exit 1`: "the CLI exited with status 1: xlsx-graft: failed to read",
	} {
		if got, _ := readFake(t, body); !reflect.DeepEqual(got, refused[any](problem("exit_status", want))) {
			t.Errorf("%s: result = %+v", body, got)
		}
	}
}

func TestOutputOutOfTheContractIsInvalidOutput(t *testing.T) {
	row := `{"row":1,"cells":[]}`
	// Each case's lines, the one quoted last: the line that broke the
	// contract, or the last line read.
	cases := map[string][]string{
		"not JSON":                      {row, "oops", endLine, "oops"},
		"not UTF-8":                     {"{\"row\":1,\"cells\":[{\"type\":\"string\",\"value\":\"\xff\"}]}", endLine, "{\"row\":1,\"cells\":[{\"type\":\"string\",\"value\":\"\xff\"}]}"},
		"no end line":                   {row, row},
		"nothing":                       {""},
		"a line after the end":          {endLine, row, row},
		"an end without rows":           {`{"ok":true,"sheet":null,"error":null,"diagnostics":[]}`},
		"an end without sheet":          {`{"ok":true,"rows":0,"error":null,"diagnostics":[]}`},
		"a diagnostic without fix":      {`{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":[{"code":"c","severity":"error","message":"m","args":{},"location":{},"decidedBy":null}]}`},
		"args that are no object":       {`{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":[{"code":"c","severity":"error","message":"m","args":[],"location":{},"decidedBy":null,"fix":null}]}`},
		"a diagnostic that is null":     {`{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":[null]}`},
		"an error that is no object":    {`{"ok":false,"sheet":null,"rows":0,"error":"x","diagnostics":[]}`},
		"an error without code":         {`{"ok":false,"sheet":null,"rows":0,"error":{"message":"m"},"diagnostics":[]}`},
		"ok that is no boolean":         {`{"ok":1,"sheet":null,"rows":0,"error":null,"diagnostics":[]}`},
		"diagnostics that are no list":  {`{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":{}}`},
		"a row number that is no int":   {`{"row":"1","cells":[]}`, endLine, `{"row":"1","cells":[]}`},
		"a row number that is null":     {`{"row":null,"cells":[]}`, endLine, `{"row":null,"cells":[]}`},
		"a line that is no object":      {"[1]", endLine, "[1]"},
		"a line that is null":           {"null", endLine, "null"},
		"a cell without value":          {`{"row":1,"cells":[{"type":"empty"}]}`, endLine, `{"row":1,"cells":[{"type":"empty"}]}`},
		"a cell without type":           {`{"row":1,"cells":[{"value":null}]}`, endLine, `{"row":1,"cells":[{"value":null}]}`},
		"an unknown cell without value": {`{"row":1,"cells":[{"type":"future"}]}`, endLine, `{"row":1,"cells":[{"type":"future"}]}`},
	}
	for kind, value := range map[string]string{
		"empty": "1", "string": "1", "number": `"3"`, "boolean": "1", "date": `"2024-1-31"`, "time": `"12:00:00"`,
		"datetime": `"2024-01-31 12:00:00.000"`, "elapsed": "1.5", "error": "null",
	} {
		line := `{"row":1,"cells":[{"type":"` + kind + `","value":` + value + `}]}`
		cases["a "+kind+" of another type"] = []string{line, endLine, line}
	}
	for name, printed := range cases {
		quoted := printed[len(printed)-1]
		if len(printed) > 1 {
			printed = printed[:len(printed)-1]
		}
		if quoted == "" {
			printed = nil
		}
		got, _ := readFake(t, lines(printed...)+"exit 0")

		if want := refused[any](invalidOutput("the end line last", quoted)); !reflect.DeepEqual(got, want) {
			t.Errorf("%s: result = %+v, want %+v", name, got.Error, want.Error)
		}
	}
}

func TestAValueThisPackageCannotGiveStopsTheSheet(t *testing.T) {
	for line, want := range map[string]string{
		`{"row":4,"cells":[{"type":"future","value":1}]}`:                 "row 4: the CLI returned a value of type future, which this package does not know",
		`{"row":4,"cells":[{"type":"elapsed","value":9300000000000000}]}`: "row 4: an elapsed time of 9300000000000000 ms is beyond a time.Duration's 292 years",
	} {
		got, read := readFake(t, lines(`{"row":1,"cells":[]}`, line, `{"row":5,"cells":[]}`, endLine)+"exit 0")

		code := "unknown_value"
		if strings.Contains(line, "elapsed") {
			code = "datetime_unrepresentable"
		}
		if !reflect.DeepEqual(got, refused[any](problem(code, want))) || !reflect.DeepEqual(read, []Row{{1, []any{}}}) {
			t.Errorf("result = %+v, rows %+v", got, read)
		}
	}
}

func TestAnElapsedTimeIsADurationEitherWay(t *testing.T) {
	got, read := readFake(t, lines(`{"row":1,"cells":[{"type":"elapsed","value":-1500},{"type":"elapsed","value":-9223372036854}]}`, endLine)+"exit 0")

	if !got.Success || !reflect.DeepEqual(read, []Row{{1, []any{-1500 * time.Millisecond, -9223372036854 * time.Millisecond}}}) {
		t.Errorf("result = %+v, rows %+v", got, read)
	}
}

func TestStoppingEarlyStopsTheCLIAndIsNoFailureWhateverItEnds(t *testing.T) {
	// Rows for as long as stdout takes them.
	forever := `while printf '%s\n' '{"row":1,"cells":[]}'; do :; done`
	for _, body := range []string{forever, lines(`{"row":1,"cells":[]}`) + "exit 3", lines(`{"row":1,"cells":[]}`) + "kill -9 $$"} {
		t.Setenv(envVar, "")
		got, err := Open(context.Background(), "book.xlsx", func(book *Book) (int, error) {
			rows := book.SheetAt(0).Rows()
			defer rows.Close()
			rows.Next()
			return rows.Row().Number, nil
		}, WithCLI(fake(t, body)))

		if want := ok(1); err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("%s: result = %+v, %v", body, got, err)
		}
	}
}

func TestADiagnosticIsKeptAsOftenAsOneRunGaveIt(t *testing.T) {
	d := func(code string) string {
		return `{"code":"` + code + `","severity":"notice","message":"m","args":{},"location":{},"decidedBy":null,"fix":null}`
	}
	listing := `{"ok":true,"sheets":[],"error":null,"diagnostics":[` + d("a") + "," + d("a") + `]}`
	end := `{"ok":true,"sheet":null,"rows":0,"error":null,"diagnostics":[` + strings.Join([]string{d("a"), d("b"), d("a"), d("a")}, ",") + `]}`
	t.Setenv(envVar, "")
	cli := fake(t, `if [ "$1" = sheets ]; then `+lines(listing)+"else "+lines(end)+"fi")
	got, err := Open(context.Background(), "book.xlsx", func(book *Book) (int, error) {
		sheets, _ := book.Sheets()
		read, err := everyRow(book.SheetAt(0).Rows())
		return len(sheets) + len(read), err
	}, WithCLI(cli))

	var codes []string
	for _, diagnostic := range got.Diagnostics {
		codes = append(codes, diagnostic.Code)
	}
	if err != nil || !got.Success || !reflect.DeepEqual(codes, []string{"a", "a", "b", "a"}) {
		t.Errorf("result = %+v, %v", got, err)
	}
}

func TestTheSheetsOfAListing(t *testing.T) {
	listing := `{"ok":true,"sheets":[{"index":0,"name":"A","kind":"worksheet","state":"visible"},{"index":1,"name":"B","kind":"future","state":"faded"}],"error":null,"diagnostics":[]}`
	got := listFake(t, lines(listing)+"exit 0")

	want := ok([]SheetInfo{{0, "A", Worksheet, Visible}, {1, "B", SheetKindUnknown, SheetStateUnknown}})
	if !reflect.DeepEqual(got, want) {
		t.Errorf("result = %+v, want %+v", got, want)
	}
}

func TestAListingOutOfTheContractOrAFailedRunListsNoSheet(t *testing.T) {
	for body, want := range map[string]Diagnostic{
		lines(`{"ok":true,"sheets":[1],"error":null,"diagnostics":[]}`) + "exit 0":    invalidOutput("the sheets", `{"ok":true,"sheets":[1],"error":null,"diagnostics":[]}`),
		`echo "xlsx-graft: failed to open" >&2; exit 1`:                               problem("exit_status", "the CLI exited with status 1: xlsx-graft: failed to open"),
		lines(`{"ok":true,"sheets":[],"error":null,"diagnostics":[]}`) + "kill -9 $$": problem("exit_status", "the CLI was killed by signal 9"),
	} {
		if got := listFake(t, body); !reflect.DeepEqual(got, refused[[]SheetInfo](want)) {
			t.Errorf("%s: result = %+v", body, got)
		}
	}
}

func TestACLIThatCannotBeStartedIsOpensError(t *testing.T) {
	t.Setenv(envVar, "")
	for name, read := range map[string]func(*Book) (int, error){
		"sheets": func(book *Book) (int, error) {
			sheets, _ := book.Sheets()
			return len(sheets), nil
		},
		"rows": func(book *Book) (int, error) {
			rows := book.SheetAt(0).Rows()
			rows.Next()
			if !errors.As(rows.Err(), new(*CLINotFoundError)) {
				t.Errorf("Err = %v", rows.Err())
			}
			return 0, nil
		},
	} {
		got, err := Open(context.Background(), "book.xlsx", read, WithCLI("/no/such/xlsx-graft"))
		if !errors.As(err, new(*CLINotFoundError)) || !reflect.DeepEqual(got, Result[int]{}) {
			t.Errorf("%s: result = %+v, %v", name, got, err)
		}
	}
}

func TestNoCLIAnywhereIsOpensError(t *testing.T) {
	t.Setenv(envVar, "")
	t.Setenv("PATH", t.TempDir())
	read := func(*Book) (int, error) {
		t.Error("read ran")
		return 0, nil
	}

	if _, err := Open(context.Background(), "book.xlsx", read); !errors.As(err, new(*CLINotFoundError)) {
		t.Errorf("Open: %v", err)
	}
	if _, err := OpenBytes(context.Background(), nil, read); !errors.As(err, new(*CLINotFoundError)) {
		t.Errorf("OpenBytes: %v", err)
	}
}

func TestADoneContextKillsTheCLIAndIsOpensError(t *testing.T) {
	t.Setenv(envVar, "")
	reads := map[string]func(context.CancelFunc, *Book) (int, error){
		"rows": func(cancel context.CancelFunc, book *Book) (int, error) {
			rows := book.SheetAt(0).Rows()
			rows.Next()
			cancel()
			if rows.Next() || !errors.Is(rows.Err(), context.Canceled) {
				t.Errorf("Err = %v", rows.Err())
			}
			return rows.Row().Number, nil
		},
		"sheets": func(cancel context.CancelFunc, book *Book) (int, error) {
			time.AfterFunc(10*time.Millisecond, cancel)
			_, err := book.Sheets()
			return 0, err
		},
		"a start": func(cancel context.CancelFunc, book *Book) (int, error) {
			cancel()
			rows := book.SheetAt(0).Rows()
			rows.Next()
			return 0, rows.Err()
		},
	}
	// The CLI itself, and a script that runs it rather than exec it: killed,
	// it leaves the CLI holding stdout and stderr.
	for _, run := range []string{"exec sleep 5", "sleep 5; :"} {
		cli := fake(t, `if [ "$1" = sheets ]; then `+run+`; fi; `+lines(`{"row":1,"cells":[]}`)+run)
		for name, read := range reads {
			ctx, cancel := context.WithCancel(context.Background())
			started := time.Now()
			got, err := Open(ctx, "book.xlsx", func(book *Book) (int, error) { return read(cancel, book) }, WithCLI(cli))
			cancel()
			if !errors.Is(err, context.Canceled) || !reflect.DeepEqual(got, Result[int]{}) || time.Since(started) > 4*time.Second {
				t.Errorf("%s, %s: result = %+v, %v", run, name, got, err)
			}
		}
	}
}

func TestAContextDoneAfterTheCLIEndedCutsTheRowsLeft(t *testing.T) {
	t.Setenv(envVar, "")
	// More rows than a buffered read takes at once, all written before the
	// first is read, the CLI gone when ctx is done.
	exited := filepath.Join(t.TempDir(), "exited")
	cli := fake(t, lines(append(slices.Repeat([]string{`{"row":1,"cells":[{"type":"string","value":"` + strings.Repeat("x", 100) + `"}]}`}, 100), endLine)...)+"touch '"+exited+"'")
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	got, err := Open(ctx, "book.xlsx", func(book *Book) (int, error) {
		rows := book.SheetAt(0).Rows()
		rows.Next()
		for _, err := os.Stat(exited); err != nil; _, err = os.Stat(exited) {
			time.Sleep(10 * time.Millisecond)
		}
		// The shell exits right after the file is made.
		time.Sleep(100 * time.Millisecond)
		cancel()
		read, err := everyRow(rows)
		return len(read), err
	}, WithCLI(cli))

	if !errors.Is(err, context.Canceled) || !reflect.DeepEqual(got, Result[int]{}) {
		t.Errorf("result = %+v, %v", got, err)
	}
}

func TestGoroutinesListTheSheetsOnce(t *testing.T) {
	t.Setenv(envVar, "")
	count := filepath.Join(t.TempDir(), "count")
	cli := fake(t, `echo run >> '`+count+`'; sleep 0.2; `+lines(`{"ok":true,"sheets":[],"error":null,"diagnostics":[]}`))
	_, err := Open(context.Background(), "book.xlsx", func(book *Book) (int, error) {
		var wg sync.WaitGroup
		for range 3 {
			wg.Go(func() { _, _ = book.Sheets() })
		}
		wg.Wait()
		return 0, nil
	}, WithCLI(cli))

	if runs, _ := os.ReadFile(count); err != nil || string(runs) != "run\n" {
		t.Errorf("runs %q, %v", runs, err)
	}
}
