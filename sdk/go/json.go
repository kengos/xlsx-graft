package xlsxgraft

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"unicode/utf8"
)

// Reads the JSON `xlsx-graft sheets` and `xlsx-graft read` print, holding
// each object to its contract (docs/sdk.md § Listing sheets and § Reading
// rows): a key missing or of another type is an error, which the caller
// turns into `invalid_output`.

// object is one JSON object the CLI printed, its values read as they are
// asked for. A map rather than a struct, whose fields encoding/json would
// match whatever the key's case.
type object map[string]json.RawMessage

// objectOf is the JSON object raw is. JSON is UTF-8, and encoding/json
// would otherwise read a byte that is not as U+FFFD.
func objectOf(raw []byte) (object, error) {
	if !utf8.Valid(raw) {
		return nil, errors.New("not UTF-8")
	}
	var o object
	if err := json.Unmarshal(raw, &o); err != nil {
		return nil, err
	}
	if o == nil {
		return nil, errors.New("not a JSON object: null")
	}
	return o, nil
}

// fields reads the fields of objects, keeping the first that is not of the
// contract; once one is kept, every read gives a zero value.
type fields struct {
	err error
}

func (f *fields) fail(err error) {
	if f.err == nil {
		f.err = err
	}
}

// isNull reports whether raw is JSON's null, which encoding/json reads into
// anything without a word.
func isNull(raw json.RawMessage) bool {
	return bytes.Equal(raw, []byte("null"))
}

// get is o's key, which o must have, as a T, null not being one.
func get[T any](f *fields, o object, key string) T {
	var value T
	raw, found := o[key]
	switch {
	case f.err != nil:
	case !found:
		f.fail(fmt.Errorf("no %q", key))
	case isNull(raw):
		f.fail(fmt.Errorf("%q is null", key))
	default:
		if err := json.Unmarshal(raw, &value); err != nil {
			f.fail(fmt.Errorf("%q: %w", key, err))
		}
	}
	return value
}

// optional is o's key, which o must have, as a JSON object; nil when null.
func optional(f *fields, o object, key string) map[string]any {
	if raw, found := o[key]; found && isNull(raw) {
		return nil
	}
	return get[map[string]any](f, o, key)
}

// has checks that o has key, whatever its value.
func has(f *fields, o object, key string) {
	if _, found := o[key]; !found {
		f.fail(fmt.Errorf("no %q", key))
	}
}

// objects is o's key, an array of JSON objects, each with its JSON.
func objects(f *fields, o object, key string) ([]object, []json.RawMessage) {
	raws := get[[]json.RawMessage](f, o, key)
	list := make([]object, 0, len(raws))
	for _, raw := range raws {
		item, err := objectOf(raw)
		f.fail(err)
		list = append(list, item)
	}
	return list, raws
}

// report is the `ok`, the `error` and the `diagnostics` of the object `sheets`
// or `verify` prints, or of `read`'s end line.
type report struct {
	// ok is the object's `ok`.
	ok bool
	// refused is why the engine refused what was asked, nil when it did not.
	refused *Diagnostic
	// diagnostics are every problem the engine found, in order.
	diagnostics []Diagnostic
	// keys are the JSON each diagnostic was read from, so the same one read
	// by two runs can be told.
	keys []string
}

func reportOf(f *fields, o object) report {
	var r report
	r.ok = get[bool](f, o, "ok")
	if raw, found := o["error"]; !found || !isNull(raw) {
		refused := get[object](f, o, "error")
		d := problem(get[string](f, refused, "code"), get[string](f, refused, "message"))
		r.refused = &d
	}
	records, raws := objects(f, o, "diagnostics")
	for i, record := range records {
		r.diagnostics = append(r.diagnostics, Diagnostic{
			Code:      get[string](f, record, "code"),
			Message:   get[string](f, record, "message"),
			Severity:  get[string](f, record, "severity"),
			Args:      get[map[string]any](f, record, "args"),
			Location:  get[map[string]any](f, record, "location"),
			DecidedBy: optional(f, record, "decidedBy"),
			Fix:       optional(f, record, "fix"),
		})
		r.keys = append(r.keys, string(raws[i]))
	}
	return r
}
