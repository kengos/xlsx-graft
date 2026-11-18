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

// InvalidRequestError is returned by [Viewer.DescribeSheet] and
// [Viewer.DescribeSheetBytes] when the CLI refused an argument they passed
// (exit status 2): a sheet request the CLI does not accept, such as a range
// that is not an A1 range, a count out of its bounds, or both a sheet name and
// a sheet index.
//
// A mistake of the caller's request, not a failed view, so it is returned as
// the error beside a zero [Result].
type InvalidRequestError struct {
	// Message is the CLI's reason, which names the CLI's own options
	// (`--range`, `--max-rows`), or a fixed sentence when the CLI gave none.
	Message string
}

func (e *InvalidRequestError) Error() string {
	return e.Message
}
