package xlsxgraft

import (
	"bufio"
	"bytes"
	"context"
	"io"
	"os/exec"
)

// stream is the CLI started, its stdout read a line at a time, as `read`'s
// JSON Lines are (docs/sdk.md § Reading rows).
type stream struct {
	ctx    context.Context
	cmd    *exec.Cmd
	stdout io.ReadCloser
	lines  *bufio.Reader
	stderr bytes.Buffer
	// unhook stops ctx closing stdout.
	unhook func() bool
}

// startCLI starts the CLI with args, as runCLI runs it, but hands its stdout
// over a line at a time. Its errors are runCLI's.
func startCLI(ctx context.Context, cli string, args ...string) (*stream, error) {
	s := &stream{ctx: ctx, cmd: cliCommand(ctx, cli, args)}
	s.cmd.Stderr = &s.stderr
	// StdoutPipe fails only as Start would (no pipe to be had), so the two
	// are reported alike.
	stdout, err := s.cmd.StdoutPipe()
	if err == nil {
		err = start(ctx, s.cmd, cli)
	}
	if err != nil {
		return nil, err
	}
	s.stdout = stdout
	s.lines = bufio.NewReader(stdout)
	// The CLI killed, a line read waits on stdout until every process
	// holding it ends, as one started through a script may not: closing it
	// ends the read at once.
	s.unhook = context.AfterFunc(ctx, func() { _ = stdout.Close() })
	return s, nil
}

// line is the next line of stdout, its line feed kept, and true; or false at
// its end, or once finish closed it. A line is as long as the CLI makes it:
// a row, which is held whole anyway, and bound by the engine's limits.
func (s *stream) line() ([]byte, bool) {
	line, _ := s.lines.ReadBytes('\n')
	return line, len(line) > 0
}

// drain reads stdout to its end, each line dropped.
func (s *stream) drain() {
	for {
		if _, more := s.line(); !more {
			return
		}
	}
}

// finish closes stdout and waits for the CLI to end: a CLI still writing
// stops at the next output it flushes, finding stdout closed. It is called
// once. The error is the context's whenever ctx was done before it: the CLI
// killed, or stdout closed by ctx, which leaves no telling a run read whole
// from one cut short.
func (s *stream) finish() (run, error) {
	// The hook ran when unhook finds it gone.
	cut := !s.unhook()
	_ = s.stdout.Close()
	err := s.cmd.Wait()
	if cut || canceled(s.ctx, s.cmd.ProcessState) != nil {
		return run{}, s.ctx.Err()
	}
	return ended(s.cmd.ProcessState, err, "", s.stderr.String()), nil
}
