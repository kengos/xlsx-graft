package xlsxgraft

import (
	"context"
	"fmt"
	"os"
	"slices"
	"strings"
)

// One run of `xlsx-graft verify` for a [Verifier] (docs/sdk.md § Verifying
// (`verify`)): checking the arguments for a NUL, writing the temporary files,
// building the CLI's arguments, and reading the result the CLI printed.

// verifyPath verifies the workbook at path.
func (v *Verifier) verifyPath(ctx context.Context, path string, rules VerifyRules) (Result[struct{}], error) {
	if strings.ContainsRune(path, 0) {
		return refused[struct{}](problem("invalid_arg", "a path holds no NUL")), nil
	}
	if rules.hasNUL() {
		return refused[struct{}](problem("invalid_arg", "an argument holds no NUL")), nil
	}
	return v.run(ctx, path, rules)
}

// verifyData verifies the workbook data holds, written to a temporary file
// first, which is removed when the run ends.
func (v *Verifier) verifyData(ctx context.Context, data []byte, rules VerifyRules) (Result[struct{}], error) {
	if rules.hasNUL() {
		return refused[struct{}](problem("invalid_arg", "an argument holds no NUL")), nil
	}
	path, err := writeTemporary(os.TempDir(), "xlsx-graft-*.xlsx", data)
	if err != nil {
		return refused[struct{}](problem("io", fmt.Sprintf("the workbook could not be written to a temporary file (%v)", err))), nil
	}
	defer func() { _ = os.Remove(path) }()
	return v.run(ctx, path, rules)
}

// run runs the CLI on the workbook at path, with the policy written to a
// temporary file of its own, which is removed when the run ends.
func (v *Verifier) run(ctx context.Context, path string, rules VerifyRules) (Result[struct{}], error) {
	args := []string{"verify", "--format", "json"}
	if v.policy != nil {
		file, err := writeTemporary(os.TempDir(), "xlsx-graft-*.json", v.policy)
		if err != nil {
			return refused[struct{}](problem("io", fmt.Sprintf("the policy could not be written to a temporary file (%v)", err))), nil
		}
		defer func() { _ = os.Remove(file) }()
		args = append(args, "--policy="+file)
	}
	args = append(args, rules.args()...)
	ran, err := runCLI(ctx, v.cli, append(args, "--", path)...)
	if err != nil {
		return Result[struct{}]{}, err
	}
	return verified(ran), nil
}

// args are the rules as arguments of the CLI, each value in one argument
// with its flag (`--target=excel >= 2016`), so that a value starting with `-`
// is not read as a flag.
func (r VerifyRules) args() []string {
	var args []string
	for _, rule := range []struct {
		flag   string
		values []string
	}{
		{"--target=", r.Targets},
		{"--best-effort=", r.BestEffort},
		{"--allow-function=", r.AllowFunctions},
		{"--allow-part=", r.AllowParts},
	} {
		for _, value := range rule.values {
			args = append(args, rule.flag+value)
		}
	}
	if r.FailOn != "" {
		args = append(args, "--fail-on="+r.FailOn)
	}
	return args
}

// hasNUL reports whether a value of the rules holds a NUL, which cannot be
// part of an argument of a process.
func (r VerifyRules) hasNUL() bool {
	for _, values := range [][]string{r.Targets, r.BestEffort, r.AllowFunctions, r.AllowParts, {r.FailOn}} {
		if slices.ContainsFunc(values, func(value string) bool { return strings.ContainsRune(value, 0) }) {
			return true
		}
	}
	return false
}

// verified reads the result of the finished CLI run ran. The CLI prints the
// result with exit status 0, 1 and 2 alike. Any other status, or status 1 or 2
// with no JSON object holding a boolean `ok`, is `exit_status`; status 0 with
// no such object is `invalid_output`. A CLI that a signal killed, or whose
// end could not be read, has status -1.
func verified(ran run) Result[struct{}] {
	if ran.status < 0 || ran.status > 2 {
		return refused[struct{}](exitStatus(ran))
	}
	f := &fields{}
	var o object
	o, f.err = objectOf([]byte(ran.stdout))
	r := reportOf(f, o)
	switch {
	case f.err == nil && r.ok:
		return Result[struct{}]{Success: true, Diagnostics: r.diagnostics}
	case f.err == nil:
		return Result[struct{}]{Diagnostics: r.diagnostics, Error: r.refused}
	case ran.status == 0:
		return refused[struct{}](invalidOutput(`a boolean "ok"`, ran.stdout))
	default:
		return refused[struct{}](exitStatus(ran))
	}
}
