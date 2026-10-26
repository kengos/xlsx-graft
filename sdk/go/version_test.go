package xlsxgraft

import (
	"errors"
	"reflect"
	"testing"
)

func TestVersionReturnsTheEngineVersionFromTheCLI(t *testing.T) {
	engineCLI(t)
	want := ok(engineVersion(t))

	for _, opts := range [][]Option{nil, {WithCLI("/elsewhere/xlsx-graft")}} {
		got, err := Version(opts...)
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("Version = %+v, %v; want %+v", got, err, want)
		}
	}
}

func TestVersionRunsTheCLIPassedWhenXlsxGraftCLIIsUnset(t *testing.T) {
	cli := engineCLI(t)
	t.Setenv(envVar, "")

	if got, err := Version(WithCLI(cli)); err != nil || got.Value != engineVersion(t) {
		t.Errorf("Version = %+v, %v", got, err)
	}
	var notFound *CLINotFoundError
	if _, err := Version(WithCLI("/elsewhere/xlsx-graft")); !errors.As(err, &notFound) {
		t.Errorf("err = %v, want a *CLINotFoundError", err)
	}
}

func TestVersionFailsWithNoCLIAnywhere(t *testing.T) {
	t.Setenv(envVar, "")
	t.Setenv("PATH", t.TempDir())

	var notFound *CLINotFoundError
	if _, err := Version(); !errors.As(err, &notFound) {
		t.Errorf("err = %v, want a *CLINotFoundError", err)
	}
}

// versionOf runs Version on a fake CLI running body, and fails the test on an
// error: a fake CLI always starts.
func versionOf(t *testing.T, body string) Result[string] {
	t.Helper()
	t.Setenv(envVar, "")
	result, err := Version(WithCLI(fake(t, body)))
	if err != nil {
		t.Fatal(err)
	}
	return result
}

func TestVersionOfANonZeroExitIsAFailedResult(t *testing.T) {
	got := versionOf(t, `echo "xlsx-graft: broken" >&2; exit 4`)

	want := failure[string](Diagnostic{Code: "exit_status", Message: "the CLI exited with status 4: xlsx-graft: broken"})
	if !reflect.DeepEqual(got, want) || !got.Failed() {
		t.Errorf("Version = %+v, want %+v", got, want)
	}
}

func TestVersionOfACLIKilledByASignalIsAFailedResult(t *testing.T) {
	got := versionOf(t, "kill -9 $$")

	want := failure[string](Diagnostic{Code: "exit_status", Message: "the CLI was killed by signal 9"})
	if !reflect.DeepEqual(got, want) {
		t.Errorf("Version = %+v, want %+v", got, want)
	}
}

func TestVersionOfNoOutputIsAFailedResult(t *testing.T) {
	got := versionOf(t, "exit 0")

	want := failure[string](Diagnostic{Code: "invalid_output", Message: `the CLI printed no JSON object with a string "version"`})
	if !reflect.DeepEqual(got, want) {
		t.Errorf("Version = %+v, want %+v", got, want)
	}
}

func TestVersionOfOutputThatIsNotUTF8IsAFailedResult(t *testing.T) {
	got := versionOf(t, `printf '{"version":"\377"}'`)

	want := failure[string](Diagnostic{Code: "invalid_output", Message: `the CLI printed no JSON object with a string "version": {"version":" "}`})
	if !reflect.DeepEqual(got, want) {
		t.Errorf("Version = %+v, want %+v", got, want)
	}
}

func TestVersionOfOutputWithNoStringVersionIsAFailedResult(t *testing.T) {
	for _, stdout := range []string{
		"0.0.0", `["0.0.0"]`, `{"version":1}`, `{"version":null}`, `{"name":"0.0.0"}`,
		`{"VERSION":"0.0.0"}`, `"0.0.0"`, "null", `{"version":"0.0.0"}{"x":1}`,
	} {
		got := versionOf(t, "printf '%s' '"+stdout+"'")

		want := failure[string](Diagnostic{Code: "invalid_output", Message: `the CLI printed no JSON object with a string "version": ` + stdout})
		if !reflect.DeepEqual(got, want) {
			t.Errorf("%s: Version = %+v, want %+v", stdout, got, want)
		}
	}
}
