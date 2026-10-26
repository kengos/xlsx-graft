package xlsxgraft

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func envOf(vars map[string]string) func(string) string {
	return func(name string) string { return vars[name] }
}

func TestTheEnvironmentWinsOverThePathPassedAndPath(t *testing.T) {
	dir := t.TempDir()
	onPath := fakeCLI(t, dir, command, "exit 0", 0o755)

	for _, c := range []struct {
		given string
		env   map[string]string
		want  string
	}{
		{"/given", map[string]string{envVar: "/from/env", "PATH": dir}, "/from/env"},
		{"/given", map[string]string{envVar: "", "PATH": dir}, "/given"},
		{"/given", map[string]string{"PATH": dir}, "/given"},
		{"", map[string]string{envVar: "", "PATH": dir}, onPath},
	} {
		got, err := findCLI(c.given, envOf(c.env))
		if err != nil || got != c.want {
			t.Errorf("findCLI(%q, %v) = %q, %v; want %q", c.given, c.env, got, err, c.want)
		}
	}
}

func TestPathIsSearchedInOrderForAnExecutableFile(t *testing.T) {
	plain := t.TempDir()
	fakeCLI(t, plain, command, "exit 0", 0o644)
	dir := t.TempDir()
	if err := os.Mkdir(filepath.Join(dir, command), 0o755); err != nil {
		t.Fatal(err)
	}
	exe := fakeCLI(t, dir, command+".exe", "exit 0", 0o755)

	relative := t.TempDir()
	fakeCLI(t, relative, command, "exit 0", 0o755)
	t.Chdir(relative)

	path := strings.Join([]string{"", ".", "/no/such/dir", plain, dir}, string(os.PathListSeparator))
	if got, err := findCLI("", envOf(map[string]string{"PATH": path})); err != nil || got != exe {
		t.Errorf("findCLI = %q, %v; want %q", got, err, exe)
	}
}

func TestExecutableReadsTheExecuteBitOutsideWindowsAndTheExeOnWindows(t *testing.T) {
	for _, c := range []struct {
		name    string
		mode    os.FileMode
		windows bool
		want    bool
	}{
		{command, 0o755, false, true},
		{command + ".exe", 0o755, false, true},
		{command, 0o644, false, false},
		{command, os.ModeDir | 0o755, false, false},
		{command + ".exe", 0o644, true, true},
		{command + ".EXE", 0o444, true, true},
		{command, 0o755, true, false},
		{command + ".exe", os.ModeDir | 0o755, true, false},
	} {
		if got := executable(c.name, c.mode, c.windows); got != c.want {
			t.Errorf("executable(%q, %v, %v) = %v, want %v", c.name, c.mode, c.windows, got, c.want)
		}
	}
}

func TestNoCLIAnywhereFailsWithTheThreeWaysToPointAtOne(t *testing.T) {
	for _, env := range []map[string]string{{"PATH": t.TempDir()}, {}} {
		_, err := findCLI("", envOf(env))

		var notFound *CLINotFoundError
		if !errors.As(err, &notFound) {
			t.Fatalf("err = %v, want a *CLINotFoundError", err)
		}
		for _, part := range []string{
			"no xlsx-graft CLI was found.\n",
			"never downloads",
			"set XLSX_GRAFT_CLI",
			`xlsxgraft.WithCLI("/path/to/xlsx-graft")`,
			"put xlsx-graft on PATH",
		} {
			if !strings.Contains(err.Error(), part) {
				t.Errorf("%q does not say %q", err, part)
			}
		}
		if notFound.Unwrap() != nil {
			t.Errorf("Unwrap() = %v, want nil: nothing was started", notFound.Unwrap())
		}
	}
}
