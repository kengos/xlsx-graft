package xlsxgraft

// Result is what every call into the engine returns: whether the call worked,
// the value when it did, why it did not when it stopped, and the diagnostics
// either way. Value is T's zero value on a failure.
//
// Error says why a read stopped short: the workbook or a sheet asked for was
// refused, or the CLI failed. A read can fail with Error nil too: one that a
// problem in its sheet stopped, whose diagnostics hold an error, as
// `xlsx-graft read`'s `ok` is false then. A call that is not a read (the
// engine's version) says why it failed in its diagnostics alone.
type Result[T any] struct {
	Success     bool
	Value       T
	Diagnostics []Diagnostic
	Error       *Diagnostic
}

// Failed reports whether the call did not work.
func (r Result[T]) Failed() bool {
	return !r.Success
}

// Diagnostic is one thing the engine reported about a call.
//
// The engine's own diagnostics (a read's [Result.Diagnostics]) carry every
// field of the JSON every host prints, keys as written there. One this
// package makes itself, such as a CLI run that failed, is an "error" with no
// Args, Location, DecidedBy or Fix.
type Diagnostic struct {
	// Code is the stable, machine-readable part.
	Code string
	// Message is for people.
	Message string
	// Severity is "error", "warning" or "notice".
	Severity string
	// Args are the values the message names, by name.
	Args map[string]any
	// Location is where in the workbook: its part, cell or row, as the
	// engine says.
	Location map[string]any
	// DecidedBy is the rule or the source the engine followed, when it says.
	DecidedBy map[string]any
	// Fix is how to resolve it, when the caller can.
	Fix map[string]any
}

// problem is a diagnostic of this package's own.
func problem(code, message string) Diagnostic {
	return Diagnostic{Code: code, Message: message, Severity: "error"}
}

func ok[T any](value T) Result[T] {
	return Result[T]{Success: true, Value: value}
}

func failure[T any](diagnostics ...Diagnostic) Result[T] {
	return Result[T]{Diagnostics: diagnostics}
}
