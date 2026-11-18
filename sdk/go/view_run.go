package xlsxgraft

import (
	"context"
	"fmt"
	"os"
	"regexp"
	"slices"
	"strings"
)

// One run of `xlsx-graft tree` or `xlsx-graft describe-sheet` for a [Viewer]
// (docs/sdk.md § Listing parts (`tree`) and § Describing a sheet
// (`describe-sheet`)): checking the arguments for a NUL, writing the
// temporary file, and reading the result the CLI printed.

// view is one of the CLI's views: its command, and how to read the keys it
// prints beside `ok`, `error` and `diagnostics`.
type view[T any] struct {
	command string
	// requestable is whether exit status 2 is the caller's request being
	// refused (`describe-sheet`), and not a command line this package built
	// wrong (`tree`, which takes a path alone).
	requestable bool
	// expected is what the CLI's JSON object must hold, for the message of
	// `invalid_output`.
	expected string
	content  func(f *fields, o object) *T
}

var treeView = view[Tree]{
	command:  "tree",
	expected: `a boolean "ok" and the parts of the workbook`,
	content:  treeOf,
}

var gridView = view[Grid]{
	command:     "describe-sheet",
	requestable: true,
	expected:    `a boolean "ok" and the cells of the sheet`,
	content:     gridOf,
}

// viewPath shows the workbook at path.
func viewPath[T any](ctx context.Context, cli string, v view[T], flags []string, path string) (Result[*T], error) {
	if strings.ContainsRune(path, 0) {
		return refused[*T](problem("invalid_arg", "a path holds no NUL")), nil
	}
	if hasNUL(flags) {
		return refused[*T](problem("invalid_arg", "an argument holds no NUL")), nil
	}
	return viewRun(ctx, cli, v, flags, path)
}

// viewData shows the workbook data holds, written to a temporary file first,
// which is removed when the run ends.
func viewData[T any](ctx context.Context, cli string, v view[T], flags []string, data []byte) (Result[*T], error) {
	if hasNUL(flags) {
		return refused[*T](problem("invalid_arg", "an argument holds no NUL")), nil
	}
	path, err := writeTemporary(os.TempDir(), "xlsx-graft-*.xlsx", data)
	if err != nil {
		return refused[*T](problem("io", fmt.Sprintf("the workbook could not be written to a temporary file (%v)", err))), nil
	}
	defer func() { _ = os.Remove(path) }()
	return viewRun(ctx, cli, v, flags, path)
}

// viewRun runs the CLI's view on the workbook at path.
func viewRun[T any](ctx context.Context, cli string, v view[T], flags []string, path string) (Result[*T], error) {
	args := append([]string{v.command, "--format", "json"}, flags...)
	ran, err := runCLI(ctx, cli, append(args, "--", path)...)
	if err != nil {
		return Result[*T]{}, err
	}
	return viewed(ran, v)
}

// hasNUL reports whether one of the arguments holds a NUL, which cannot be
// part of an argument of a process.
func hasNUL(args []string) bool {
	return slices.ContainsFunc(args, func(arg string) bool { return strings.ContainsRune(arg, 0) })
}

// viewed reads the result of the finished CLI run ran. The CLI prints the
// result with exit status 0 and 1 alike. Status 2 of a view whose request can
// be refused is an [*InvalidRequestError]. Any other status, or status 1 with
// no JSON object holding what the view reads, is `exit_status`; status 0 with
// no such object is `invalid_output`. A CLI that a signal killed, or whose end
// could not be read, has status -1.
func viewed[T any](ran run, v view[T]) (Result[*T], error) {
	switch {
	case v.requestable && ran.status == 2:
		return Result[*T]{}, &InvalidRequestError{Message: refusal(ran.stderr)}
	case ran.status != 0 && ran.status != 1:
		return refused[*T](exitStatus(ran)), nil
	}
	f := &fields{}
	var o object
	o, f.err = objectOf([]byte(ran.stdout))
	r := reportOf(f, o)
	content := v.content(f, o)
	switch {
	case f.err == nil && r.ok:
		return Result[*T]{Success: true, Value: content, Diagnostics: r.diagnostics}, nil
	case f.err == nil:
		return Result[*T]{Value: content, Diagnostics: r.diagnostics, Error: r.refused}, nil
	case ran.status == 0:
		return refused[*T](invalidOutput(v.expected, ran.stdout)), nil
	default:
		return refused[*T](exitStatus(ran)), nil
	}
}

// cliName is the `xlsx-graft:` the CLI starts its own messages with, and the
// white space after it, which is the whole message when the CLI gave no more.
var cliName = regexp.MustCompile(`^xlsx-graft:(\s+|$)`)

// refusal is the reason the CLI gave for refusing an argument: its message on
// stderr, without the `xlsx-graft: ` the CLI starts its own messages with.
func refusal(stderr string) string {
	reason := cliName.ReplaceAllString(strings.TrimSpace(stderr), "")
	if reason == "" {
		return "the CLI refused an argument (exit status 2) and gave no reason"
	}
	return reason
}
