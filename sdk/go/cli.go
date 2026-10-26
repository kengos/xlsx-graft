package xlsxgraft

import (
	"io/fs"
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

// The CLI is looked up in this order, and the first one set wins (a set path
// that cannot be run fails when it is run, rather than falls through; a set
// name with no directory in it is looked up on PATH by os/exec):
//
//  1. the XLSX_GRAFT_CLI environment variable
//  2. the path the caller passed ([WithCLI])
//  3. xlsx-graft on PATH
//
// The environment comes first because where the engine lives is the
// deployment's decision, and it has to be able to override application code.
// Nothing here downloads anything.
const (
	envVar  = "XLSX_GRAFT_CLI"
	command = "xlsx-graft"
)

// findCLI returns the CLI getenv's XLSX_GRAFT_CLI or given names, or the
// first on getenv's PATH.
func findCLI(given string, getenv func(string) string) (string, error) {
	// An empty value counts as unset, as it does for the other SDKs.
	for _, set := range []string{getenv(envVar), given} {
		if set != "" {
			return set, nil
		}
	}
	if found := onPath(getenv("PATH")); found != "" {
		return found, nil
	}
	return "", &CLINotFoundError{Message: installHint("no xlsx-graft CLI was found")}
}

// onPath searches path by hand rather than through exec.LookPath, which reads
// the process's own PATH, so the environment can be handed in (the tests do),
// and so a CLI that is not there is reported as missing rather than as a
// failed run. Windows' .exe spelling is tried at every entry, on every system.
// An entry that is not absolute (empty, or `.`) is skipped: os/exec refuses
// to run what it would find there (exec.ErrDot).
func onPath(path string) string {
	for _, dir := range filepath.SplitList(path) {
		if !filepath.IsAbs(dir) {
			continue
		}
		for _, name := range []string{command, command + ".exe"} {
			candidate := filepath.Join(dir, name)
			if info, err := os.Stat(candidate); err == nil && executable(name, info.Mode(), runtime.GOOS == "windows") {
				return candidate
			}
		}
	}
	return ""
}

// executable reports whether the file name, of mode, can be run as the CLI.
// windows is a parameter so both halves are tested where the gate runs.
// Windows has no execute bit (os.Stat reports 0666 or 0444 there), and runs a
// name with no extension by trying PATHEXT's, a .cmd among them, which would
// hand the arguments to cmd.exe (docs/sdk.md § Running the CLI): there, only
// an .exe is the CLI.
func executable(name string, mode fs.FileMode, windows bool) bool {
	if windows {
		return mode.IsRegular() && strings.EqualFold(filepath.Ext(name), ".exe")
	}
	return mode.IsRegular() && mode.Perm()&0o111 != 0
}

// installHint is why the CLI cannot be reached, followed by the three ways
// to point this package at one.
func installHint(reason string) string {
	return strings.Join([]string{
		reason + ".",
		"",
		"This package never downloads the engine. Point it at one of these ways:",
		"  * set " + envVar + " to the path of an xlsx-graft executable",
		`  * pass xlsxgraft.WithCLI("/path/to/xlsx-graft")`,
		"  * put " + command + " on PATH",
	}, "\n")
}
