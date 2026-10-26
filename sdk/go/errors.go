package xlsxgraft

// CLINotFoundError is returned when no engine CLI could be found, or the one
// found cannot be run.
//
// A setup mistake, not a failed call, so it is returned as the error beside a
// [Result] rather than as a diagnostic in one.
type CLINotFoundError struct {
	// Message says why, then the ways to point this package at a CLI.
	Message string
	// Err is what starting the CLI failed with, if it was started at all.
	Err error
}

func (e *CLINotFoundError) Error() string {
	return e.Message
}

// Unwrap returns what starting the CLI failed with, so errors.Is can ask,
// say, whether the file does not exist.
func (e *CLINotFoundError) Unwrap() error {
	return e.Err
}
