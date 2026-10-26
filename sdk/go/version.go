package xlsxgraft

import (
	"encoding/json"
	"os"
	"unicode/utf8"
)

// Version returns the engine's version, such as "0.0.0", on success, by
// running `xlsx-graft --version --format json`.
//
// The error is a [*CLINotFoundError] when no CLI is found or it cannot be
// started; a run that gave no version is a failed [Result] instead.
func Version(opts ...Option) (Result[string], error) {
	cli, err := findCLI(apply(opts).cli, os.Getenv)
	if err != nil {
		return Result[string]{}, err
	}
	r, err := runCLI(cli, "--version", "--format", "json")
	if err != nil {
		return Result[string]{}, err
	}
	if r.status != 0 || r.signal != 0 {
		return failure[string](exitStatus(r)), nil
	}
	// JSON is UTF-8, and encoding/json would otherwise read a byte that is
	// not as U+FFFD. A map rather than a struct, whose fields encoding/json
	// would match whatever the key's case.
	var printed map[string]any
	err = json.Unmarshal([]byte(r.stdout), &printed)
	version, isString := printed["version"].(string)
	if !utf8.ValidString(r.stdout) || err != nil || !isString {
		return failure[string](invalidOutput(`a string "version"`, r.stdout)), nil
	}
	return ok(version), nil
}
