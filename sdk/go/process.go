package xlsxgraft

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"time"
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
// It returns a [*CLINotFoundError] when the CLI cannot be started, and ctx's
// error when ctx ended the run, killing the CLI.
func runCLI(ctx context.Context, cli string, args ...string) (run, error) {
	var stdout, stderr bytes.Buffer
	cmd := cliCommand(ctx, cli, args)
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	if err := start(ctx, cmd, cli); err != nil {
		return run{}, err
	}
	err := cmd.Wait()
	if cut := canceled(ctx, cmd.ProcessState); cut != nil {
		return run{}, cut
	}
	return ended(cmd.ProcessState, err, stdout.String(), stderr.String()), nil
}

// waitDelay is how long a run its context killed, or that ended, may hold
// its pipes open after: a CLI started through a script that does not exec it
// leaves the engine holding them, killed or not. os/exec closes them then.
const waitDelay = time.Second

// cliCommand is the CLI run with args, killed when ctx is done. Killing loses
// nothing: the files the CLI spools to are unnamed, so the system removes
// them however it ends (engine/spool).
func cliCommand(ctx context.Context, cli string, args []string) *exec.Cmd {
	cmd := exec.CommandContext(ctx, withExe(cli, runtime.GOOS == "windows"), args...)
	cmd.WaitDelay = waitDelay
	return cmd
}

// start starts cmd, the CLI at cli; or returns ctx's error when ctx is done,
// and a [*CLINotFoundError] when the CLI cannot be started.
func start(ctx context.Context, cmd *exec.Cmd, cli string) error {
	err := cmd.Start()
	switch {
	case err == nil:
		return nil
	case ctx.Err() != nil:
		return ctx.Err()
	default:
		return &CLINotFoundError{Message: installHint(fmt.Sprintf("%s could not be run (%v)", cli, err)), Err: err}
	}
}

// canceled is ctx's error when ctx is done and the run, whose end is state,
// did not end well: ctx killed it, or may have. os/exec reports a run it
// killed by the signal, not by ctx's error.
func canceled(ctx context.Context, state *os.ProcessState) error {
	if ctx.Err() != nil && (state == nil || !state.Success()) {
		return ctx.Err()
	}
	return nil
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
