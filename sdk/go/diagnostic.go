package xlsxgraft

import (
	"fmt"
	"strings"
	"unicode"
	"unicode/utf8"
)

// The diagnostics of a CLI run that did not give a result (docs/sdk.md §
// Running the CLI). The codes are provisional, as the other SDKs' status codes
// are: the engine's own diagnostics replace them once the CLI reports some.

// quoteLimit is how much of the CLI's output a message quotes, in characters.
const quoteLimit = 200

// exitStatus is the CLI exiting with a status other than 0, a signal killing
// it, or its end not read at all.
func exitStatus(r run) Diagnostic {
	reason := fmt.Sprintf("the CLI exited with status %d", r.status)
	switch {
	case r.lost != nil:
		reason = fmt.Sprintf("the CLI's end could not be read (%v)", r.lost)
	case r.signal != 0:
		reason = fmt.Sprintf("the CLI was killed by signal %d", r.signal)
	}
	return Diagnostic{Code: "exit_status", Message: quoting(reason, r.stderr)}
}

// invalidOutput is the CLI exiting with 0, but printing no JSON object with
// expected, what the call reads from it (`a string "version"`).
func invalidOutput(expected, stdout string) Diagnostic {
	return Diagnostic{Code: "invalid_output", Message: quoting("the CLI printed no JSON object with "+expected, stdout)}
}

// quoting is reason, followed by the start of output when there is any.
//
// Runs of what is not graphic (control and format characters, line and
// paragraph separators) become one space, so the quote stays on one line, and
// output that is not UTF-8 keeps its printable ASCII alone. The cut is in
// characters, never through one.
func quoting(reason, output string) string {
	keep := unicode.IsGraphic
	if !utf8.ValidString(output) {
		keep = func(r rune) bool { return r >= 0x20 && r <= 0x7e }
	}
	text := strings.Join(strings.FieldsFunc(output, func(r rune) bool { return !keep(r) }), " ")
	text = strings.Trim(text, " ")
	if text == "" {
		return reason
	}
	if runes := []rune(text); len(runes) > quoteLimit {
		text = string(runes[:quoteLimit]) + "..."
	}
	return reason + ": " + text
}
