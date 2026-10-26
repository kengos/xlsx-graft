package xlsxgraft

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
)

// run is how one run of the CLI ended.
type run struct {
	// status is the exit status, -1 when a signal killed the CLI.
	status int
	// signal is the signal that killed it, 0 when it exited.
	signal int
	// lost is why the end could not be read at all, nil when it was.
	lost   error
	stdout string
	stderr string
}

// runCLI runs the engine's CLI once (docs/sdk.md § Running the CLI).
//
// The command is an argument list, so the CLI is started itself and no shell
// reads the arguments. A nil Stdin is the null device, so a CLI that ever
// asked for input reads its end rather than waiting. os/exec reads stdout and
// stderr at once, so neither fills its pipe while the other is read.
//
// It returns a [*CLINotFoundError] when the CLI cannot be started.
func runCLI(cli string, args ...string) (run, error) {
	var stdout, stderr bytes.Buffer
	cmd := exec.Command(withExe(cli, runtime.GOOS == "windows"), args...)
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	if err := cmd.Start(); err != nil {
		return run{}, &CLINotFoundError{Message: installHint(fmt.Sprintf("%s could not be run (%v)", cli, err)), Err: err}
	}
	err := cmd.Wait()
	return ended(cmd.ProcessState, err, stdout.String(), stderr.String()), nil
}

// withExe is cli with .exe added on Windows when it has no extension. os/exec
// would otherwise try PATHEXT's extensions, .cmd among them, which hands the
// arguments to cmd.exe (docs/sdk.md § Running the CLI). windows is a
// parameter so both halves are tested where the gate runs.
func withExe(cli string, windows bool) string {
	if windows && filepath.Ext(cli) == "" {
		return cli + ".exe"
	}
	return cli
}

// ended is the run whose end Wait read into state, or failed to read with
// err. Wait's error is otherwise the exit status, which state holds, or a
// failure to copy the output, which a bytes.Buffer never gives. state is nil
// when the system reaped the CLI before Wait could, as it does for an
// application that ignores SIGCHLD.
func ended(state *os.ProcessState, err error, stdout, stderr string) run {
	if state == nil {
		return run{status: -1, lost: err, stdout: stdout, stderr: stderr}
	}
	return run{status: state.ExitCode(), signal: signalOf(state), stdout: stdout, stderr: stderr}
}
