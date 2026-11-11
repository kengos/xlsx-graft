package xlsxgraft

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
)

// Verifier verifies workbooks under the host's policy, given to
// [NewVerifier], and the rules of the caller, given to each call, by running
// `xlsx-graft verify` (docs/sdk.md § Verifying in an SDK).
//
// A Verifier holds the CLI's path and the policy's JSON, and neither changes
// after [NewVerifier] returns, so one Verifier may be used by any number of
// goroutines at once. Each call writes the temporary files it needs under
// names of its own.
//
//	verifier, err := xlsxgraft.NewVerifier(map[string]any{"maxEntries": 1000})
//	if err != nil {
//		return err
//	}
//	result, err := verifier.Verify(ctx, "book.xlsx", xlsxgraft.VerifyRules{
//		Targets:    []string{"excel >= 2016"},
//		AllowParts: []string{"macro"},
//	})
type Verifier struct {
	cli string
	// policy is the policy as JSON, nil when there is none.
	policy []byte
}

// VerifyRules are the rules of one call, each optional: a nil or empty slice
// and an empty string are not given. Nothing here is checked by this package:
// the engine judges each value, and a fault is a `target_invalid` or
// `rules_invalid` diagnostic of the result.
type VerifyRules struct {
	// Targets are the versions the workbook must run in (`excel >= 2016`);
	// a problem with one is an error.
	Targets []string
	// BestEffort are the versions it should run in; a problem with one is a
	// warning.
	BestEffort []string
	// AllowFunctions are functions refused by default to let through,
	// without an `_xlfn.` prefix.
	AllowFunctions []string
	// AllowParts are kinds of part refused by default to let through
	// (`macro`, `external_link`, `dde`, `ole`, `activex`, `data_connection`,
	// `web_publish`).
	AllowParts []string
	// FailOn is "warn" to fail on a warning too, "error" for the default;
	// the stricter of this and the policy's `failOn` counts.
	FailOn string
}

// NewVerifier makes a Verifier for policy: the engine's limits, `failOn`,
// `denyScheduled` and `forbid` (`functions` and `parts`), by the engine's
// camelCase names, which are not changed. The same policy file as the CLI's
// `--policy` is a map when read with [json.Unmarshal]. A nil map is the
// engine's defaults; an empty one is a policy of no key.
//
// The policy is not checked here: the engine checks it when it verifies, and
// a policy it does not take gives a failed [Result] of `policy_invalid`
// diagnostics at the first call. The Verifier keeps the policy's JSON at the
// time NewVerifier runs, so changing the map afterwards has no effect. A
// string of the policy that is not UTF-8 is written as U+FFFD in its place
// (json.Marshal does), and the engine judges the text it gets.
//
// The error is a [*CLINotFoundError] when no CLI is found (the CLI is looked
// up as [Open] looks it up; [WithLocation] has no effect), or an error
// wrapping json.Marshal's when the policy cannot be written as JSON (NaN, an
// infinity, a channel, a function).
func NewVerifier(policy map[string]any, opts ...Option) (*Verifier, error) {
	cli, err := findCLI(apply(opts).cli, os.Getenv)
	if err != nil {
		return nil, err
	}
	v := &Verifier{cli: cli}
	if policy != nil {
		v.policy, err = json.Marshal(policy)
		if err != nil {
			return nil, fmt.Errorf("xlsxgraft: the policy cannot be written as JSON: %w", err)
		}
	}
	return v, nil
}

// Verify verifies the workbook at path.
//
// The result is a success when the engine's `ok` is true; it has no Value. A
// workbook that cannot be read, or a temporary file that cannot be written,
// is a failure whose Error is `io`; a fault of the policy or the rules is a
// `policy_invalid`, `rules_invalid` or `target_invalid` diagnostic, and the
// workbook is not opened. A path or a rule holding a NUL is `invalid_arg`, and
// the CLI is not run. With FailOn "warn" (or a policy's) a result fails on a
// warning with no error diagnostic. A CLI that gave no result is a failure
// of `exit_status` or `invalid_output`.
//
// The error is a [*CLINotFoundError] when the CLI cannot be started, or ctx's
// error when ctx ended the run (killing the CLI); the result is then the zero
// Result.
func (v *Verifier) Verify(ctx context.Context, path string, rules VerifyRules) (Result[struct{}], error) {
	return v.verifyPath(ctx, path, rules)
}

// VerifyBytes is as [Verifier.Verify], the workbook being data, written to a
// temporary file of the call's own (the CLI reads a path) that only its user
// may read, removed when the call returns. Changing data once the SDK has
// written the file has no effect, even while the call runs. A file that
// cannot be written is `io`.
func (v *Verifier) VerifyBytes(ctx context.Context, data []byte, rules VerifyRules) (Result[struct{}], error) {
	return v.verifyData(ctx, data, rules)
}
