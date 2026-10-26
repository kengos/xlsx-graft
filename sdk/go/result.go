package xlsxgraft

// Result is what every call into the engine returns: whether the call worked,
// the value when it did, and the diagnostics either way. Value is T's zero
// value on a failure.
type Result[T any] struct {
	Success     bool
	Value       T
	Diagnostics []Diagnostic
}

// Failed reports whether the call did not work.
func (r Result[T]) Failed() bool {
	return !r.Success
}

// Diagnostic is one thing the engine reported about a call.
type Diagnostic struct {
	// Code is the stable, machine-readable part.
	Code string
	// Message is for people.
	Message string
}

func ok[T any](value T) Result[T] {
	return Result[T]{Success: true, Value: value}
}

func failure[T any](diagnostics ...Diagnostic) Result[T] {
	return Result[T]{Diagnostics: diagnostics}
}
