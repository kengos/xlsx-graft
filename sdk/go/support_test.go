package xlsxgraft

import (
	"os"
	"path/filepath"
	"regexp"
	"testing"
)

// What the tests share: the engine's CLI, the engine's version, and fake CLIs
// for the runs the real one never gives.

// engineCLI is the CLI `make engine:cli-bin` built; the gate points
// XLSX_GRAFT_CLI at it.
func engineCLI(t *testing.T) string {
	t.Helper()
	path := os.Getenv(envVar)
	if path == "" {
		t.Fatal(envVar + " is unset; run the tests through make sdk:test:go")
	}
	return path
}

// engineVersion is the engine workspace's version, from engine/Cargo.toml.
func engineVersion(t *testing.T) string {
	t.Helper()
	cargoToml, err := os.ReadFile("../../engine/Cargo.toml")
	if err != nil {
		t.Fatal(err)
	}
	found := regexp.MustCompile(`(?ms)^\[workspace\.package\][^\[]*?^version = "([^"]+)"`).FindSubmatch(cargoToml)
	if found == nil {
		t.Fatal("engine/Cargo.toml has no [workspace.package] version")
	}
	return string(found[1])
}

// fakeCLI writes name into dir: a shell script running body, with mode.
func fakeCLI(t *testing.T, dir, name, body string, mode os.FileMode) string {
	t.Helper()
	path := filepath.Join(dir, name)
	if err := os.WriteFile(path, []byte("#!/bin/sh\n"+body+"\n"), mode); err != nil {
		t.Fatal(err)
	}
	return path
}

// fake is a runnable fake CLI named xlsx-graft in a directory of its own.
func fake(t *testing.T, body string) string {
	t.Helper()
	return fakeCLI(t, t.TempDir(), command, body, 0o755)
}
