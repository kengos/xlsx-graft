package xlsxgraft

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

// What a Viewer makes of the runs the real CLI never gives, and the arguments
// it runs the CLI with, from fake CLIs (docs/sdk.md § Listing parts (`tree`)
// and § Describing a sheet (`describe-sheet`)).

const (
	emptyTree = `{"ok":true,"error":null,"diagnostics":[],"complete":true,"relationshipsKnown":true,"relationships":[],"parts":[]}`
	emptyGrid = `{"ok":true,"error":null,"diagnostics":[],"sheet":null,"dimension":null,"used":null,"shown":null,` +
		`"truncated":{"rows":false,"columns":false,"merges":false},"complete":true,"rows":[],"merges":[]}`
)

// fakeViewer is a Viewer whose CLI runs body when it is asked for a view.
func fakeViewer(t *testing.T, body string) *Viewer {
	t.Helper()
	t.Setenv(envVar, "")
	viewer, err := NewViewer(WithCLI(fake(t, "[ \"$1\" = tree ] || [ \"$1\" = describe-sheet ] || exit 99\n"+body)))
	if err != nil {
		t.Fatal(err)
	}
	return viewer
}

// recordingView is a fake CLI's body that writes its arguments, one a line,
// to args, then prints printed.
func recordingView(args, printed string) string {
	return `printf '%s\n' "$@" > '` + args + `'
printf '%s\n' '` + printed + `'`
}

func argumentsOf(t *testing.T, path string) []string {
	t.Helper()
	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return strings.Split(strings.TrimSuffix(string(got), "\n"), "\n")
}

func TestTheCLIIsRunWithTheViewsArgumentsEachValueInOneArgument(t *testing.T) {
	dir := t.TempDir()
	args := filepath.Join(dir, "args")
	ctx := context.Background()

	tree, treeErr := fakeViewer(t, recordingView(args, emptyTree)).Tree(ctx, "-book.xlsx")
	treeArgs := argumentsOf(t, args)
	grid, gridErr := fakeViewer(t, recordingView(args, emptyGrid)).DescribeSheet(ctx, "-book.xlsx", SheetRequest{
		Sheet: "-x", SheetIndex: ptr(2), Range: "A1:B2", MaxRows: 5, MaxColumns: 7,
	})
	gridArgs := argumentsOf(t, args)
	_, noneErr := fakeViewer(t, recordingView(args, emptyGrid)).DescribeSheet(ctx, "book.xlsx", SheetRequest{})
	noneArgs := argumentsOf(t, args)
	_, zeroErr := fakeViewer(t, recordingView(args, emptyGrid)).DescribeSheet(ctx, "book.xlsx", SheetRequest{SheetIndex: ptr(0)})
	zeroArgs := argumentsOf(t, args)

	if treeErr != nil || gridErr != nil || noneErr != nil || zeroErr != nil || !tree.Success || !grid.Success {
		t.Fatalf("tree %+v (%v), grid %+v (%v), %v, %v", tree, treeErr, grid, gridErr, noneErr, zeroErr)
	}
	if want := []string{"tree", "--format", "json", "--", "-book.xlsx"}; !reflect.DeepEqual(treeArgs, want) {
		t.Errorf("tree args = %q", treeArgs)
	}
	want := []string{
		"describe-sheet", "--format", "json", "--sheet=-x", "--sheet-index=2", "--range=A1:B2",
		"--max-rows=5", "--max-columns=7", "--", "-book.xlsx",
	}
	if !reflect.DeepEqual(gridArgs, want) {
		t.Errorf("describe-sheet args = %q", gridArgs)
	}
	if want := []string{"describe-sheet", "--format", "json", "--", "book.xlsx"}; !reflect.DeepEqual(noneArgs, want) {
		t.Errorf("args of an empty request = %q", noneArgs)
	}
	if want := []string{"describe-sheet", "--format", "json", "--sheet-index=0", "--", "book.xlsx"}; !reflect.DeepEqual(zeroArgs, want) {
		t.Errorf("args of a sheet index of 0 = %q", zeroArgs)
	}
}

func TestTheBytesAreWrittenToATemporaryFileThatIsRemovedAfterTheRun(t *testing.T) {
	copied := filepath.Join(t.TempDir(), "copied")
	body := `for a; do p="$a"; done; cat "$p" > '` + copied + `'; printf '%s\n' '` + emptyTree + `'`
	viewer := fakeViewer(t, body)
	tmp := t.TempDir()
	t.Setenv("TMPDIR", tmp)

	result, err := viewer.TreeBytes(context.Background(), []byte("the data"))

	got, _ := os.ReadFile(copied)
	left, _ := os.ReadDir(tmp)
	if err != nil || !result.Success || string(got) != "the data" || len(left) != 0 {
		t.Errorf("result %+v (%v), copy %q, left behind %v", result, err, got, left)
	}
}

func TestAViewArgumentHoldingANULIsInvalidArgAndTheCLIIsNotRun(t *testing.T) {
	marker := filepath.Join(t.TempDir(), "run")
	viewer := fakeViewer(t, "touch '"+marker+"'\nprintf '%s\\n' '"+emptyGrid+"'")
	ctx := context.Background()
	for name, tc := range map[string]struct {
		path    string
		request SheetRequest
		message string
	}{
		"a path":       {"a\x00b", SheetRequest{}, "a path holds no NUL"},
		"a sheet name": {"book.xlsx", SheetRequest{Sheet: "a\x00"}, "an argument holds no NUL"},
		"a range":      {"book.xlsx", SheetRequest{Range: "A1\x00"}, "an argument holds no NUL"},
	} {
		want := &Diagnostic{Code: "invalid_arg", Message: tc.message, Severity: "error"}
		grid, err := viewer.DescribeSheet(ctx, tc.path, tc.request)
		if err != nil || grid.Success || grid.Value != nil || !reflect.DeepEqual(grid.Error, want) {
			t.Errorf("%s: grid %+v (%v)", name, grid, err)
		}
		if tc.message == "an argument holds no NUL" {
			data, err := viewer.DescribeSheetBytes(ctx, nil, tc.request)
			if err != nil || data.Success || data.Value != nil || !reflect.DeepEqual(data.Error, want) {
				t.Errorf("%s by data: %+v (%v)", name, data, err)
			}
		}
	}
	tree, err := viewer.Tree(ctx, "a\x00b")
	if err != nil || tree.Success || tree.Value != nil || tree.Error == nil || tree.Error.Code != "invalid_arg" {
		t.Errorf("tree %+v (%v)", tree, err)
	}
	if _, err := os.Stat(marker); !errors.Is(err, os.ErrNotExist) {
		t.Errorf("the CLI was run (%v)", err)
	}
}

func TestATemporaryFileThatCannotBeWrittenIsIoWithNoValue(t *testing.T) {
	marker := filepath.Join(t.TempDir(), "run")
	viewer := fakeViewer(t, "touch '"+marker+"'")
	t.Setenv("TMPDIR", filepath.Join(t.TempDir(), "missing"))

	tree, treeErr := viewer.TreeBytes(context.Background(), nil)
	grid, gridErr := viewer.DescribeSheetBytes(context.Background(), nil, SheetRequest{})

	if treeErr != nil || gridErr != nil || tree.Success || tree.Value != nil || grid.Success || grid.Value != nil {
		t.Fatalf("tree %+v (%v), grid %+v (%v)", tree, treeErr, grid, gridErr)
	}
	for _, d := range []*Diagnostic{tree.Error, grid.Error} {
		if d == nil || d.Code != "io" || !strings.HasPrefix(d.Message, "the workbook could not be written to a temporary file (") {
			t.Errorf("error = %+v", d)
		}
	}
	if _, err := os.Stat(marker); !errors.Is(err, os.ErrNotExist) {
		t.Errorf("the CLI was run (%v)", err)
	}
}

func TestARunThatGaveNoResultIsExitStatusOrInvalidOutputWithNoValue(t *testing.T) {
	for name, tc := range map[string]struct {
		body string
		code string
		text string
	}{
		"exit 3":                        {lines(emptyTree) + "exit 3", "exit_status", "the CLI exited with status 3"},
		"a signal":                      {"kill -9 $$", "exit_status", "the CLI was killed by signal 9"},
		"exit 1 with no JSON":           {"echo not json; echo broken >&2; exit 1", "exit_status", "the CLI exited with status 1: broken"},
		"exit 1 without the parts":      {lines(`{"ok":false,"error":null,"diagnostics":[]}`) + "exit 1", "exit_status", "the CLI exited with status 1"},
		"exit 2, whose JSON is no use":  {lines(emptyTree) + "exit 2", "exit_status", "the CLI exited with status 2"},
		"exit 0 with no JSON":           {"echo hello", "invalid_output", `the CLI printed no JSON object with a boolean "ok" and the parts of the workbook: hello`},
		"exit 0 with no ok":             {lines(`{"diagnostics":[]}`), "invalid_output", `a boolean "ok"`},
		"exit 0 with parts of a string": {lines(strings.Replace(emptyTree, `"parts":[]`, `"parts":"x"`, 1)), "invalid_output", `the parts of the workbook`},
	} {
		viewer := fakeViewer(t, tc.body)

		result, err := viewer.Tree(context.Background(), "book.xlsx")

		if err != nil || result.Success || result.Value != nil || len(result.Diagnostics) != 0 || result.Error == nil ||
			result.Error.Code != tc.code || !strings.Contains(result.Error.Message, tc.text) {
			t.Errorf("%s: result = %+v (%v), error %+v", name, result, err, result.Error)
		}
	}
	grid, err := fakeViewer(t, "echo hello").DescribeSheet(context.Background(), "book.xlsx", SheetRequest{})
	if err != nil || grid.Value != nil || grid.Error == nil || !strings.Contains(grid.Error.Message, "the cells of the sheet: hello") {
		t.Errorf("grid %+v (%v)", grid, err)
	}
}

func TestAViewRunWhoseEndWasNotReadIsExitStatus(t *testing.T) {
	result, err := viewed(run{status: -1, lost: errors.New("no wait"), stdout: emptyTree}, treeView)

	if err != nil || result.Success || result.Value != nil || result.Error == nil ||
		result.Error.Message != "the CLI's end could not be read (no wait)" {
		t.Errorf("result %+v (%v)", result, err)
	}
}

func TestExitStatus2OfDescribeSheetIsAnInvalidRequestErrorWithTheCLIsReason(t *testing.T) {
	for name, tc := range map[string]struct {
		stderr  string
		message string
	}{
		"with the CLI's name":                {"xlsx-graft: --range \"A0\" is not an A1 range\\n", `--range "A0" is not an A1 range`},
		"without the CLI's name":             {"error: unexpected argument\\n\\nUsage: xlsx-graft", "error: unexpected argument\n\nUsage: xlsx-graft"},
		"with only spaces":                   {" \\n", "the CLI refused an argument (exit status 2) and gave no reason"},
		"with nothing":                       {"", "the CLI refused an argument (exit status 2) and gave no reason"},
		"with only the CLI's name":           {"xlsx-graft: \\n", "the CLI refused an argument (exit status 2) and gave no reason"},
		"with a name that is part of a word": {"xlsx-graft:x", "xlsx-graft:x"},
	} {
		viewer := fakeViewer(t, "printf '"+tc.stderr+"' >&2; exit 2")

		result, err := viewer.DescribeSheet(context.Background(), "book.xlsx", SheetRequest{})

		var refused *InvalidRequestError
		if !errors.As(err, &refused) || refused.Message != tc.message || !reflect.DeepEqual(result, Result[*Grid]{}) {
			t.Errorf("%s: result %+v, error %v", name, result, err)
		}
	}
}

func TestASuccessfulViewKeepsItsValueAndItsDiagnostics(t *testing.T) {
	warned := strings.Replace(emptyTree, `"diagnostics":[]`, `"diagnostics":[`+
		`{"code":"x","message":"m","severity":"warn","args":{},"location":{},"decidedBy":null,"fix":null}]`, 1)

	result, err := viewed(run{status: 0, stdout: warned}, treeView)

	if err != nil || !result.Success || result.Value == nil || result.Error != nil ||
		len(result.Diagnostics) != 1 || result.Diagnostics[0].Code != "x" || result.Diagnostics[0].Severity != "warn" {
		t.Errorf("result %+v (%v)", result, err)
	}
}

func TestAFailedViewKeepsItsValueItsErrorAndItsDiagnostics(t *testing.T) {
	failed := `{"ok":false,"error":{"code":"io","message":"gone"},"diagnostics":[` +
		`{"code":"x","message":"m","severity":"warn","args":{},"location":{},"decidedBy":null,"fix":null}],` +
		`"complete":false,"relationshipsKnown":false,"relationships":[],"parts":[]}`
	errorDiagnostic := `{"code":"x","message":"m","severity":"error","args":{},"location":{},"decidedBy":null,"fix":null}`
	nonError := strings.Replace(strings.Replace(emptyTree, `"ok":true`, `"ok":false`, 1), `"diagnostics":[]`, `"diagnostics":[`+errorDiagnostic+`]`, 1)
	for name, tc := range map[string]struct {
		status int
		stdout string
	}{"io": {1, failed}, "an error diagnostic and no error": {1, nonError}, "exit 0": {0, failed}} {
		result, err := viewed(run{status: tc.status, stdout: tc.stdout}, treeView)

		if err != nil || result.Success || result.Value == nil || (result.Error == nil) != (tc.stdout == nonError) {
			t.Errorf("%s: result %+v (%v)", name, result, err)
		}
		if tc.stdout == nonError && len(result.Diagnostics) != 1 {
			t.Errorf("%s: result %+v", name, result)
		}
		if tc.stdout == failed && (result.Error.Code != "io" || len(result.Diagnostics) != 1 || result.Value.Complete) {
			t.Errorf("%s: result %+v", name, result)
		}
	}
}

func TestAViewCLIThatCannotBeStartedIsACLINotFoundErrorOfTheCall(t *testing.T) {
	t.Setenv(envVar, "")
	viewer, err := NewViewer(WithCLI(filepath.Join(t.TempDir(), "nowhere")))
	if err != nil {
		t.Fatal(err)
	}

	result, err := viewer.Tree(context.Background(), "book.xlsx")

	var notFound *CLINotFoundError
	if !errors.As(err, &notFound) || !reflect.DeepEqual(result, Result[*Tree]{}) {
		t.Errorf("result %+v, error %v", result, err)
	}
}

func TestACanceledContextEndsTheRunWithItsErrorAndLeavesNoTemporaryFile(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("TMPDIR", dir)
	viewer := fakeViewer(t, "sleep 30")
	ctx, cancel := context.WithCancel(context.Background())
	time.AfterFunc(300*time.Millisecond, cancel)

	result, err := viewer.DescribeSheetBytes(ctx, []byte("data"), SheetRequest{})

	left, _ := os.ReadDir(dir)
	if !errors.Is(err, context.Canceled) || !reflect.DeepEqual(result, Result[*Grid]{}) || len(left) != 0 {
		t.Errorf("result %+v, error %v, left behind %v", result, err, left)
	}
}

func TestTheTemporaryFileOfTheBytesCanBeReadByItsUserAlone(t *testing.T) {
	listing := filepath.Join(t.TempDir(), "listing")
	body := `for a; do p="$a"; done; ls -l "$p" | cut -c1-10 > '` + listing + `'; printf '%s\n' '` + emptyTree + `'`
	viewer := fakeViewer(t, body)

	result, err := viewer.TreeBytes(context.Background(), []byte("the data"))

	got, _ := os.ReadFile(listing)
	if err != nil || !result.Success || strings.TrimSpace(string(got)) != "-rw-------" {
		t.Errorf("result %+v (%v), listing %q", result, err, got)
	}
}
