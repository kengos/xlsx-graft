package xlsxgraft

import (
	"errors"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestRunReturnsTheStatusStdoutAndStderrApart(t *testing.T) {
	cli := fake(t, `printf "%s|" "$@"; echo oops >&2; exit 3`)

	got, err := runCLI(cli, "--version", "a b", "$HOME")
	want := run{status: 3, stdout: "--version|a b|$HOME|", stderr: "oops\n"}
	if err != nil || got != want {
		t.Errorf("runCLI = %+v, %v; want %+v", got, err, want)
	}
}

func TestRunGivesStdinItsEndAtOnce(t *testing.T) {
	// `cat` would wait for ever on an open stdin.
	got, err := runCLI(fake(t, "cat; echo done"))
	if want := (run{stdout: "done\n"}); err != nil || got != want {
		t.Errorf("runCLI = %+v, %v; want %+v", got, err, want)
	}
}

func TestRunDrainsAStderrLargerThanAPipeHolds(t *testing.T) {
	got, err := runCLI(fake(t, "head -c 200000 /dev/zero >&2; echo out"))
	if err != nil || got.status != 0 || got.stdout != "out\n" || len(got.stderr) != 200000 {
		t.Errorf("runCLI = status %d, stdout %q, %d bytes of stderr, %v", got.status, got.stdout, len(got.stderr), err)
	}
}

func TestRunTellsASignalFromAStatus(t *testing.T) {
	killed, err := runCLI(fake(t, "kill -9 $$"))
	if want := (run{status: -1, signal: 9}); err != nil || killed != want {
		t.Errorf("killed: runCLI = %+v, %v; want %+v", killed, err, want)
	}
	exited, err := runCLI(fake(t, "exit 9"))
	if want := (run{status: 9}); err != nil || exited != want {
		t.Errorf("exited: runCLI = %+v, %v; want %+v", exited, err, want)
	}
}

func TestWithExeAddsTheExeOnWindowsOnly(t *testing.T) {
	for _, c := range []struct {
		cli     string
		windows bool
		want    string
	}{
		{"/tools/xlsx-graft", true, "/tools/xlsx-graft.exe"},
		{"xlsx-graft", true, "xlsx-graft.exe"},
		{"/tools.d/xlsx-graft", true, "/tools.d/xlsx-graft.exe"},
		{"/tools/xlsx-graft.exe", true, "/tools/xlsx-graft.exe"},
		{"/tools/xlsx-graft.cmd", true, "/tools/xlsx-graft.cmd"},
		{"/tools/xlsx-graft", false, "/tools/xlsx-graft"},
	} {
		if got := withExe(c.cli, c.windows); got != c.want {
			t.Errorf("withExe(%q, %v) = %q, want %q", c.cli, c.windows, got, c.want)
		}
	}
}

func TestAnEndWaitCouldNotReadIsAFailedRun(t *testing.T) {
	lost := errors.New("wait: no child processes")

	got := ended(nil, lost, "out", "err")
	if want := (run{status: -1, lost: lost, stdout: "out", stderr: "err"}); got != want {
		t.Errorf("ended = %+v, want %+v", got, want)
	}
}

func TestRunLooksANameWithNoDirectoryUpOnPath(t *testing.T) {
	got, err := runCLI("sh", "-c", "echo on path")
	if want := (run{stdout: "on path\n"}); err != nil || got != want {
		t.Errorf("runCLI = %+v, %v; want %+v", got, err, want)
	}
	var notFound *CLINotFoundError
	if _, err := runCLI("no-such-xlsx-graft"); !errors.As(err, &notFound) {
		t.Errorf("err = %v, want a *CLINotFoundError", err)
	}
}

func TestRunFailsForACLIThatCannotBeStarted(t *testing.T) {
	dir := t.TempDir()
	plain := fakeCLI(t, dir, command, "exit 0", 0o644)
	missing := filepath.Join(dir, "missing")

	for _, cli := range []string{plain, missing, dir} {
		_, err := runCLI(cli, "--version")

		var notFound *CLINotFoundError
		if !errors.As(err, &notFound) {
			t.Fatalf("%s: err = %v, want a *CLINotFoundError", cli, err)
		}
		if !strings.HasPrefix(err.Error(), cli+" could not be run (") || !strings.Contains(err.Error(), "set XLSX_GRAFT_CLI") {
			t.Errorf("%s: %q", cli, err)
		}
	}
	if _, err := runCLI(missing); !errors.Is(err, fs.ErrNotExist) {
		t.Errorf("err = %v, want one that is fs.ErrNotExist", err)
	}
	if _, err := runCLI(plain); !errors.Is(err, os.ErrPermission) {
		t.Errorf("err = %v, want one that is os.ErrPermission", err)
	}
}
