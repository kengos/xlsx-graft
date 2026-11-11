package xlsxgraft

import (
	"context"
	"os"
	"path/filepath"
	"reflect"
	"slices"
	"strings"
	"sync"
	"testing"
)

// A Verifier against the real CLI (docs/sdk.md § Verifying in an SDK), on the
// workbooks of fixtures/.

const (
	macro   = "../../fixtures/macro.xlsm"
	table   = "../../fixtures/table.xlsx"
	dynamic = "../../fixtures/dynamic-array.xlsx"
)

// engineVerifier is a Verifier for policy that runs the engine's CLI.
func engineVerifier(t *testing.T, policy map[string]any) *Verifier {
	t.Helper()
	verifier, err := NewVerifier(policy, WithCLI(engineCLI(t)))
	if err != nil {
		t.Fatal(err)
	}
	return verifier
}

func verify(t *testing.T, verifier *Verifier, path string, rules VerifyRules) Result[struct{}] {
	t.Helper()
	result, err := verifier.Verify(context.Background(), path, rules)
	if err != nil {
		t.Fatal(err)
	}
	return result
}

func verifyBytes(t *testing.T, verifier *Verifier, data []byte, rules VerifyRules) Result[struct{}] {
	t.Helper()
	result, err := verifier.VerifyBytes(context.Background(), data, rules)
	if err != nil {
		t.Fatal(err)
	}
	return result
}

func codesOf(result Result[struct{}]) []string {
	var codes []string
	for _, d := range result.Diagnostics {
		codes = append(codes, d.Code)
	}
	return codes
}

func severitiesOf(result Result[struct{}]) []string {
	var severities []string
	for _, d := range result.Diagnostics {
		severities = append(severities, d.Severity)
	}
	return severities
}

func TestAWorkbookTheEngineRefusesFailsWithItsDiagnosticsAndNoError(t *testing.T) {
	result := verify(t, engineVerifier(t, nil), macro, VerifyRules{})

	if result.Success || result.Error != nil || result.Value != (struct{}{}) {
		t.Errorf("result = %+v", result)
	}
	if want := []string{"part_denied", "part_denied"}; !reflect.DeepEqual(codesOf(result), want) {
		t.Fatalf("codes = %v, want %v", codesOf(result), want)
	}
	first := result.Diagnostics[0]
	if first.Severity != "error" || len(first.Args) == 0 || first.DecidedBy == nil || first.Fix["in"] != "template" {
		t.Errorf("first diagnostic = %+v", first)
	}
}

func TestAPartTheRulesAllowAndAWorkbookWithNoFaultPass(t *testing.T) {
	verifier := engineVerifier(t, nil)

	allowed := verify(t, verifier, macro, VerifyRules{AllowParts: []string{"macro"}})
	plain := verify(t, verifier, table, VerifyRules{})

	if !allowed.Success || len(allowed.Diagnostics) != 0 || !plain.Success || len(plain.Diagnostics) != 0 {
		t.Errorf("allowed %+v, plain %+v", allowed, plain)
	}
}

func TestAFileThatCannotBeReadFailsWithAnIoError(t *testing.T) {
	result := verify(t, engineVerifier(t, nil), "/nowhere/book.xlsx", VerifyRules{})

	if result.Success || len(result.Diagnostics) != 0 || result.Error == nil || result.Error.Code != "io" ||
		!strings.Contains(result.Error.Message, "/nowhere/book.xlsx") {
		t.Errorf("result = %+v, error %+v", result, result.Error)
	}
}

func TestAFaultOfTheRulesIsADiagnosticAndNoError(t *testing.T) {
	verifier := engineVerifier(t, nil)
	for name, tc := range map[string]struct {
		rules VerifyRules
		code  string
	}{
		"an unknown target":               {VerifyRules{Targets: []string{"excel 1990"}}, "target_invalid"},
		"a target starting with a dash":   {VerifyRules{Targets: []string{"-x"}}, "target_invalid"},
		"an allowed function with prefix": {VerifyRules{AllowFunctions: []string{"_xlfn.SEQUENCE"}}, "rules_invalid"},
	} {
		result := verify(t, verifier, "/nowhere/book.xlsx", tc.rules)

		if result.Success || result.Error != nil || !slices.Contains(codesOf(result), tc.code) {
			t.Errorf("%s: result = %+v, error %+v", name, result, result.Error)
		}
	}
}

func TestFailOnWarnFailsAWarningOnlyWhenTheRulesSaySo(t *testing.T) {
	verifier := engineVerifier(t, nil)
	rules := VerifyRules{BestEffort: []string{"excel 2016"}}

	lenient := verify(t, verifier, dynamic, rules)
	rules.FailOn = "warn"
	strict := verify(t, verifier, dynamic, rules)

	if !lenient.Success || !slices.Contains(severitiesOf(lenient), "warn") {
		t.Errorf("lenient = %+v", lenient)
	}
	if strict.Success || strict.Error != nil || slices.Contains(severitiesOf(strict), "error") {
		t.Errorf("strict = %+v", strict)
	}
}

func TestAFailOnTheCLIRefusesIsExitStatus(t *testing.T) {
	result := verify(t, engineVerifier(t, nil), table, VerifyRules{FailOn: "nope"})

	if result.Success || len(result.Diagnostics) != 0 || result.Error == nil || result.Error.Code != "exit_status" {
		t.Errorf("result = %+v, error %+v", result, result.Error)
	}
}

func TestARuleThatIsNotUTF8IsExitStatus(t *testing.T) {
	result := verify(t, engineVerifier(t, nil), table, VerifyRules{Targets: []string{"\xff"}})

	if result.Success || len(result.Diagnostics) != 0 || result.Error == nil || result.Error.Code != "exit_status" {
		t.Errorf("result = %+v, error %+v", result, result.Error)
	}
}

func TestVerifyBytesVerifiesTheDataAndDataThatIsNoWorkbookFails(t *testing.T) {
	verifier := engineVerifier(t, nil)
	data, err := os.ReadFile(table)
	if err != nil {
		t.Fatal(err)
	}
	macroData, err := os.ReadFile(macro)
	if err != nil {
		t.Fatal(err)
	}

	good := verifyBytes(t, verifier, data, VerifyRules{})
	allowed := verifyBytes(t, verifier, macroData, VerifyRules{AllowParts: []string{"macro"}})
	empty := verifyBytes(t, verifier, nil, VerifyRules{})

	if !good.Success || !allowed.Success {
		t.Errorf("good %+v, allowed %+v", good, allowed)
	}
	if empty.Success || empty.Error != nil || len(empty.Diagnostics) == 0 {
		t.Errorf("empty = %+v", empty)
	}
}

func TestThePolicysMaxEntriesFailsTheVerification(t *testing.T) {
	result := verify(t, engineVerifier(t, map[string]any{"maxEntries": 1}), table, VerifyRules{})

	if result.Success || len(result.Diagnostics) == 0 || result.Diagnostics[0].DecidedBy["source"] != "policy" {
		t.Errorf("result = %+v", result)
	}
}

func TestThePolicysFailOnMakesAWarningFail(t *testing.T) {
	verifier := engineVerifier(t, map[string]any{"failOn": "warn"})

	result := verify(t, verifier, dynamic, VerifyRules{BestEffort: []string{"excel 2016"}})

	if result.Success {
		t.Errorf("result = %+v", result)
	}
}

func TestAPolicyTheEngineDoesNotTakeFailsEveryCallWithPolicyInvalid(t *testing.T) {
	policy := map[string]any{
		"maxEntries": 0,
		"forbid":     map[string]any{"parts": []any{map[string]any{"kind": "nope"}}},
	}
	verifier := engineVerifier(t, policy)

	data, err := os.ReadFile(table)
	if err != nil {
		t.Fatal(err)
	}

	for name, result := range map[string]Result[struct{}]{
		"Verify":      verify(t, verifier, table, VerifyRules{}),
		"VerifyBytes": verifyBytes(t, verifier, data, VerifyRules{}),
	} {
		var keys []any
		for _, d := range result.Diagnostics {
			if d.Code != "policy_invalid" {
				t.Errorf("%s: diagnostic = %+v", name, d)
			}
			keys = append(keys, d.Args["key"])
		}
		if result.Success || result.Error != nil || !reflect.DeepEqual(keys, []any{"maxEntries", "forbid.parts[0]"}) {
			t.Errorf("%s: result = %+v, keys %v", name, result, keys)
		}
	}
}

func TestChangingThePolicyMapAfterwardsChangesNothing(t *testing.T) {
	policy := map[string]any{"maxEntries": 1}
	verifier := engineVerifier(t, policy)
	policy["maxEntries"] = 10000

	if result := verify(t, verifier, table, VerifyRules{}); result.Success {
		t.Errorf("result = %+v", result)
	}
}

func TestOneVerifierServesGoroutinesThatCallItAtOnce(t *testing.T) {
	verifier := engineVerifier(t, map[string]any{"failOn": "error"})
	data, err := os.ReadFile(macro)
	if err != nil {
		t.Fatal(err)
	}
	codes := make([][]string, 4)
	var wg sync.WaitGroup
	for i := range codes {
		wg.Add(1)
		go func() {
			defer wg.Done()
			result, err := verifier.VerifyBytes(context.Background(), data, VerifyRules{})
			if err != nil {
				t.Error(err)
			}
			codes[i] = codesOf(result)
		}()
	}
	wg.Wait()

	for i, got := range codes {
		if want := []string{"part_denied", "part_denied"}; !reflect.DeepEqual(got, want) {
			t.Errorf("goroutine %d: codes = %v", i, got)
		}
	}
}

func TestNoTemporaryFileIsLeftBehind(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("TMPDIR", dir)
	verifier := engineVerifier(t, map[string]any{"failOn": "error"})
	data, err := os.ReadFile(table)
	if err != nil {
		t.Fatal(err)
	}

	byPath := verify(t, verifier, table, VerifyRules{})
	byData := verifyBytes(t, verifier, data, VerifyRules{})

	left, _ := filepath.Glob(filepath.Join(dir, "xlsx-graft-*"))
	if !byPath.Success || !byData.Success || len(left) != 0 {
		t.Errorf("path %+v, data %+v, left behind %v", byPath, byData, left)
	}
}
