package xlsxgraft

import (
	"context"
	"encoding/json"
	"errors"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"strings"
	"testing"
	"time"
)

// What a Verifier makes of the runs the real CLI never gives, and the
// arguments it runs the CLI with, from fake CLIs (docs/sdk.md § Verifying
// (`verify`)).

const okObject = `{"ok":true,"failOn":"error","counts":{"error":0,"warn":0,"notice":0},"error":null,"diagnostics":[]}`

// fakeVerifier is a Verifier for policy whose CLI runs body.
func fakeVerifier(t *testing.T, policy map[string]any, body string) *Verifier {
	t.Helper()
	t.Setenv(envVar, "")
	verifier, err := NewVerifier(policy, WithCLI(fake(t, "[ \"$1\" = verify ] || exit 99\n"+body)))
	if err != nil {
		t.Fatal(err)
	}
	return verifier
}

// recording is a fake CLI's body that writes its arguments, one a line, to
// args, and the policy file it was given, if any, to policy; then prints
// okObject.
func recording(args, policy string) string {
	return `printf '%s\n' "$@" > '` + args + `'
for a; do case "$a" in --policy=*) cat "${a#--policy=}" > '` + policy + `';; esac; done
printf '%s\n' '` + okObject + `'`
}

func TestTheRulesAndThePolicyGoToTheCLIEachValueInOneArgument(t *testing.T) {
	dir := t.TempDir()
	args, policy := filepath.Join(dir, "args"), filepath.Join(dir, "policy")
	verifier := fakeVerifier(t, map[string]any{"maxEntries": 5, "forbid": map[string]any{}}, recording(args, policy))

	result := verify(t, verifier, "-book.xlsx", VerifyRules{
		Targets:        []string{"-x", "excel >= 2016"},
		BestEffort:     []string{"excel 2013"},
		AllowFunctions: []string{"SEQUENCE"},
		AllowParts:     []string{"macro", "ole"},
		FailOn:         "warn",
	})

	got, _ := os.ReadFile(args)
	lines := strings.Split(strings.TrimSuffix(string(got), "\n"), "\n")
	file := strings.TrimPrefix(lines[3], "--policy=")
	if !regexp.MustCompile(`/xlsx-graft-\d+\.json$`).MatchString(file) {
		t.Errorf("policy file = %q", file)
	}
	lines[3] = "--policy=FILE"
	want := []string{
		"verify", "--format", "json", "--policy=FILE", "--target=-x", "--target=excel >= 2016",
		"--best-effort=excel 2013", "--allow-function=SEQUENCE", "--allow-part=macro", "--allow-part=ole",
		"--fail-on=warn", "--", "-book.xlsx",
	}
	written, _ := os.ReadFile(policy)
	_, stillThere := os.Stat(file)
	if !result.Success || !reflect.DeepEqual(lines, want) || string(written) != `{"forbid":{},"maxEntries":5}` || !errors.Is(stillThere, os.ErrNotExist) {
		t.Errorf("result %+v, args %q, policy %q, file left: %v", result, lines, written, stillThere)
	}
}

func TestRulesNotGivenAndNoPolicyMakeNoArguments(t *testing.T) {
	dir := t.TempDir()
	args := filepath.Join(dir, "args")
	verifier := fakeVerifier(t, nil, recording(args, filepath.Join(dir, "policy")))

	verify(t, verifier, "book.xlsx", VerifyRules{Targets: []string{}})

	got, _ := os.ReadFile(args)
	if want := "verify\n--format\njson\n--\nbook.xlsx\n"; string(got) != want {
		t.Errorf("args = %q, want %q", got, want)
	}
}

func TestAnEmptyPolicyIsWrittenAsAnEmptyObject(t *testing.T) {
	dir := t.TempDir()
	policy := filepath.Join(dir, "policy")
	verifier := fakeVerifier(t, map[string]any{}, recording(filepath.Join(dir, "args"), policy))

	verify(t, verifier, "book.xlsx", VerifyRules{})

	if written, _ := os.ReadFile(policy); string(written) != "{}" {
		t.Errorf("policy = %q", written)
	}
}

func TestVerifyBytesGivesTheCLIAFileHoldingTheData(t *testing.T) {
	dir := t.TempDir()
	copied := filepath.Join(dir, "copied")
	verifier := fakeVerifier(t, nil, `for a; do p="$a"; done; cat "$p" > '`+copied+`'; printf '%s\n' '`+okObject+`'`)
	tmp := t.TempDir()
	t.Setenv("TMPDIR", tmp)

	result := verifyBytes(t, verifier, []byte("the data"), VerifyRules{})

	got, _ := os.ReadFile(copied)
	left, _ := os.ReadDir(tmp)
	if !result.Success || string(got) != "the data" || len(left) != 0 {
		t.Errorf("result %+v, copy %q, left behind %v", result, got, left)
	}
}

func TestAValueHoldingANULIsInvalidArgAndTheCLIIsNotRun(t *testing.T) {
	marker := filepath.Join(t.TempDir(), "run")
	verifier := fakeVerifier(t, nil, "touch '"+marker+"'\nprintf '%s\\n' '"+okObject+"'")
	for name, tc := range map[string]struct {
		path    string
		rules   VerifyRules
		message string
	}{
		"a path":              {"a\x00b", VerifyRules{}, "a path holds no NUL"},
		"a target":            {"book.xlsx", VerifyRules{Targets: []string{"a\x00"}}, "an argument holds no NUL"},
		"a best-effort value": {"book.xlsx", VerifyRules{BestEffort: []string{"a\x00"}}, "an argument holds no NUL"},
		"an allowed function": {"book.xlsx", VerifyRules{AllowFunctions: []string{"a\x00"}}, "an argument holds no NUL"},
		"an allowed part":     {"book.xlsx", VerifyRules{AllowParts: []string{"a\x00"}}, "an argument holds no NUL"},
		"a fail-on value":     {"book.xlsx", VerifyRules{FailOn: "a\x00"}, "an argument holds no NUL"},
	} {
		want := &Diagnostic{Code: "invalid_arg", Message: tc.message, Severity: "error"}
		results := map[string]Result[struct{}]{"path": verify(t, verifier, tc.path, tc.rules)}
		if tc.message != "a path holds no NUL" {
			results["data"] = verifyBytes(t, verifier, nil, tc.rules)
		}
		for call, result := range results {
			if result.Success || !reflect.DeepEqual(result.Error, want) {
				t.Errorf("%s by %s: result = %+v", name, call, result)
			}
		}
	}
	if _, err := os.Stat(marker); !errors.Is(err, os.ErrNotExist) {
		t.Errorf("the CLI was run (%v)", err)
	}
}

func TestATemporaryFileThatCannotBeWrittenIsIo(t *testing.T) {
	marker := filepath.Join(t.TempDir(), "run")
	t.Setenv("TMPDIR", filepath.Join(t.TempDir(), "missing"))
	withPolicy := fakeVerifier(t, map[string]any{}, "touch '"+marker+"'")
	without := fakeVerifier(t, nil, "touch '"+marker+"'")

	policy := verify(t, withPolicy, "book.xlsx", VerifyRules{})
	data := verifyBytes(t, without, nil, VerifyRules{})

	if policy.Success || policy.Error == nil || policy.Error.Code != "io" ||
		!strings.HasPrefix(policy.Error.Message, "the policy could not be written to a temporary file (") {
		t.Errorf("policy: %+v", policy.Error)
	}
	if data.Success || data.Error == nil || data.Error.Code != "io" ||
		!strings.HasPrefix(data.Error.Message, "the workbook could not be written to a temporary file (") {
		t.Errorf("data: %+v", data.Error)
	}
	if _, err := os.Stat(marker); !errors.Is(err, os.ErrNotExist) {
		t.Errorf("the CLI was run (%v)", err)
	}
}

func TestARunThatGaveNoResultIsExitStatusOrInvalidOutput(t *testing.T) {
	for name, tc := range map[string]struct {
		body string
		code string
		text string
	}{
		"exit 3":                             {lines(okObject) + "exit 3", "exit_status", "the CLI exited with status 3"},
		"a signal":                           {"kill -9 $$", "exit_status", "the CLI was killed by signal 9"},
		"exit 1 with no JSON":                {"echo not json; echo broken >&2; exit 1", "exit_status", "the CLI exited with status 1: broken"},
		"exit 2 with no stdout":              {"echo 'error: unexpected argument' >&2; exit 2", "exit_status", "the CLI exited with status 2: error: unexpected argument"},
		"exit 2 with JSON but no ok":         {lines("{}") + "exit 2", "exit_status", "the CLI exited with status 2"},
		"exit 0 with no JSON":                {"echo hello", "invalid_output", `the CLI printed no JSON object with a boolean "ok": hello`},
		"exit 0 with no ok":                  {lines(`{"diagnostics":[]}`), "invalid_output", `a boolean "ok"`},
		"exit 0 with an ok that is a string": {lines(`{"ok":"yes","error":null,"diagnostics":[]}`), "invalid_output", `a boolean "ok"`},
		"exit 0 with no diagnostics":         {lines(`{"ok":true,"error":null}`), "invalid_output", `a boolean "ok"`},
	} {
		result := verify(t, fakeVerifier(t, nil, tc.body), "book.xlsx", VerifyRules{})

		if result.Success || len(result.Diagnostics) != 0 || result.Error == nil ||
			result.Error.Code != tc.code || !strings.Contains(result.Error.Message, tc.text) {
			t.Errorf("%s: result = %+v", name, result)
		}
	}
}

func TestACLIWhoseEndWasNotReadIsExitStatus(t *testing.T) {
	result := verified(run{status: -1, lost: errors.New("no wait"), stdout: okObject})

	if result.Success || result.Error == nil || result.Error.Message != "the CLI's end could not be read (no wait)" {
		t.Errorf("result = %+v", result)
	}
}

func TestAResultIsReadWhateverStatusItCameWith(t *testing.T) {
	failed := `{"ok":false,"failOn":"error","counts":{},"error":{"code":"io","message":"gone"},"diagnostics":[` +
		`{"code":"x","message":"m","severity":"warn","args":{},"location":{},"decidedBy":null,"fix":null}]}`
	for status, stdout := range map[int]string{0: okObject, 1: failed, 2: failed} {
		result := verified(run{status: status, stdout: stdout})

		if result.Success != (stdout == okObject) || (result.Error != nil) != (stdout == failed) {
			t.Errorf("status %d: result = %+v", status, result)
		}
		if stdout == failed && (result.Error.Code != "io" || len(result.Diagnostics) != 1) {
			t.Errorf("status %d: result = %+v", status, result)
		}
	}
}

func TestAPolicyThatJSONCannotWriteIsAnErrorOfNewVerifier(t *testing.T) {
	t.Setenv(envVar, "")
	for name, policy := range map[string]map[string]any{
		"NaN":      {"maxEntries": math.NaN()},
		"Inf":      {"maxEntries": math.Inf(1)},
		"a chan":   {"maxEntries": make(chan int)},
		"a func":   {"maxEntries": func() {}},
		"a nested": {"forbid": map[string]any{"parts": []any{math.NaN()}}},
	} {
		verifier, err := NewVerifier(policy, WithCLI("xlsx-graft"))

		if verifier != nil || err == nil || !strings.HasPrefix(err.Error(), "xlsxgraft: the policy cannot be written as JSON: ") {
			t.Errorf("%s: verifier %v, error %v", name, verifier, err)
		}
	}
}

func TestNoCLIFoundIsACLINotFoundErrorOfNewVerifier(t *testing.T) {
	t.Setenv(envVar, "")
	t.Setenv("PATH", t.TempDir())

	verifier, err := NewVerifier(map[string]any{})

	var notFound *CLINotFoundError
	if verifier != nil || !errors.As(err, &notFound) {
		t.Errorf("verifier %v, error %v", verifier, err)
	}
}

func TestACLIThatCannotBeStartedIsACLINotFoundErrorOfTheCall(t *testing.T) {
	t.Setenv(envVar, "")
	verifier, err := NewVerifier(nil, WithCLI(filepath.Join(t.TempDir(), "nowhere")))
	if err != nil {
		t.Fatal(err)
	}

	result, err := verifier.Verify(context.Background(), "book.xlsx", VerifyRules{})

	var notFound *CLINotFoundError
	if !errors.As(err, &notFound) || !reflect.DeepEqual(result, Result[struct{}]{}) {
		t.Errorf("result %+v, error %v", result, err)
	}
}

func TestACanceledContextEndsTheRunWithItsError(t *testing.T) {
	verifier := fakeVerifier(t, nil, "sleep 30")
	ctx, cancel := context.WithCancel(context.Background())
	time.AfterFunc(300*time.Millisecond, cancel)

	result, err := verifier.Verify(ctx, "book.xlsx", VerifyRules{})

	if !errors.Is(err, context.Canceled) || !reflect.DeepEqual(result, Result[struct{}]{}) {
		t.Errorf("result %+v, error %v", result, err)
	}
}

func TestACanceledRunLeavesNoTemporaryFile(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("TMPDIR", dir)
	verifier := fakeVerifier(t, map[string]any{"maxEntries": 5}, "sleep 30")
	ctx, cancel := context.WithCancel(context.Background())
	time.AfterFunc(300*time.Millisecond, cancel)

	_, err := verifier.VerifyBytes(ctx, []byte("data"), VerifyRules{})

	left, _ := os.ReadDir(dir)
	if !errors.Is(err, context.Canceled) || len(left) != 0 {
		t.Errorf("error %v, left behind %v", err, left)
	}
}

func TestAPolicyStringThatIsNotUTF8IsWrittenWithTheReplacementCharacter(t *testing.T) {
	dir := t.TempDir()
	policy := filepath.Join(dir, "policy")
	verifier := fakeVerifier(t, map[string]any{
		"forbid": map[string]any{"functions": []any{map[string]any{"name": "\xff"}}},
	}, recording(filepath.Join(dir, "args"), policy))

	result := verify(t, verifier, "book.xlsx", VerifyRules{})

	// The Go releases write U+FFFD as an escape or as the character, so the
	// JSON is read rather than compared.
	written, _ := os.ReadFile(policy)
	var read struct {
		Forbid struct {
			Functions []struct{ Name string }
		}
	}
	err := json.Unmarshal(written, &read)
	if !result.Success || err != nil || len(read.Forbid.Functions) != 1 || read.Forbid.Functions[0].Name != "\ufffd" {
		t.Errorf("result %+v, policy %q (%v)", result, written, err)
	}
}
