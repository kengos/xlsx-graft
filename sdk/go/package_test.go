package xlsxgraft

import (
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"regexp"
	"slices"
	"strconv"
	"strings"
	"testing"
)

// The package downloads nothing and depends on nothing (docs/sdk.md § Finding
// the engine library).

// networking is an import of a package that reaches the network.
func networking(path string) bool {
	return path == "net" || strings.HasPrefix(path, "net/") || path == "crypto/tls"
}

func TestUsesNoNetworking(t *testing.T) {
	sources, err := filepath.Glob("*.go")
	if err != nil {
		t.Fatal(err)
	}
	// A sweep over no files reports no offenders and proves nothing.
	if !slices.Contains(sources, "process.go") {
		t.Fatalf("the sweep reads %v, not process.go", sources)
	}

	for _, source := range sources {
		if strings.HasSuffix(source, "_test.go") {
			continue
		}
		file, err := parser.ParseFile(token.NewFileSet(), source, nil, parser.ImportsOnly)
		if err != nil {
			t.Fatal(err)
		}
		for _, imported := range file.Imports {
			if path, _ := strconv.Unquote(imported.Path.Value); networking(path) {
				t.Errorf("%s imports %s", source, path)
			}
		}
	}
}

func TestTheSweepMatchesNetworking(t *testing.T) {
	for path, want := range map[string]bool{"net": true, "net/http": true, "crypto/tls": true, "os/exec": false, "network": false} {
		if networking(path) != want {
			t.Errorf("networking(%q) = %v, want %v", path, !want, want)
		}
	}
}

func TestRequiresNothing(t *testing.T) {
	goMod, err := os.ReadFile("go.mod")
	if err != nil {
		t.Fatal(err)
	}
	if regexp.MustCompile(`(?m)^\s*(require|tool)\b`).Match(goMod) {
		t.Errorf("go.mod requires something:\n%s", goMod)
	}
}
