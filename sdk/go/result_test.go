package xlsxgraft

import "testing"

func TestResultFailed(t *testing.T) {
	if ok("0.0.0").Failed() {
		t.Error("a successful result failed")
	}
	if !failure[string](Diagnostic{Code: "c", Message: "m"}).Failed() {
		t.Error("a failed result did not fail")
	}
}
