package xlsxgraft

import (
	"errors"
	"strings"
	"testing"
)

func TestExitStatusQuotesStderr(t *testing.T) {
	for _, c := range []struct {
		run  run
		want string
	}{
		{run{status: 2, stderr: "xlsx-graft: error: unexpected argument\n\nUsage: ...\n"}, "the CLI exited with status 2: xlsx-graft: error: unexpected argument Usage: ..."},
		{run{status: 1, stderr: " \n"}, "the CLI exited with status 1"},
		{run{status: -1, signal: 9}, "the CLI was killed by signal 9"},
		{run{status: -1, lost: errors.New("wait: no child processes"), stderr: "oops"}, "the CLI's end could not be read (wait: no child processes): oops"},
	} {
		if got := exitStatus(c.run); got != (Diagnostic{Code: "exit_status", Message: c.want}) {
			t.Errorf("exitStatus(%+v) = %+v, want %q", c.run, got, c.want)
		}
	}
}

func TestInvalidOutputQuotesStdout(t *testing.T) {
	for stdout, want := range map[string]string{
		"0.0.0\n": `the CLI printed no JSON object with a string "version": 0.0.0`,
		"":        `the CLI printed no JSON object with a string "version"`,
	} {
		if got := invalidOutput(`a string "version"`, stdout); got != (Diagnostic{Code: "invalid_output", Message: want}) {
			t.Errorf("invalidOutput(%q) = %+v, want %q", stdout, got, want)
		}
	}
}

func TestQuotingCutsInCharacters(t *testing.T) {
	for output, want := range map[string]string{
		strings.Repeat("x", 200): strings.Repeat("x", 200),
		strings.Repeat("x", 201): strings.Repeat("x", 200) + "...",
		strings.Repeat("表", 250): strings.Repeat("表", 200) + "...",
	} {
		if got := quoting("r", output); got != "r: "+want {
			t.Errorf("quoting(%d bytes) = %q", len(output), got)
		}
	}
}

func TestQuotingKeepsTheQuoteOnOneLine(t *testing.T) {
	if got := quoting("r", "a\r\nb\u2028c\u2029d\u202e\t"); got != "r: a b c d" {
		t.Errorf("quoting = %q", got)
	}
}

func TestQuotingKeepsPrintableASCIIOfWhatIsNotUTF8(t *testing.T) {
	if got := quoting("r", "bad\xff\xfebytes\x00here"); got != "r: bad bytes here" {
		t.Errorf("quoting = %q", got)
	}
}
