package xlsxgraft

import (
	"context"
	"errors"
	"os"
	"slices"
	"sync"
	"time"
)

// ErrClosed is what a [Book] and its [Rows] give once the function [Open]
// handed the book to has returned.
var ErrClosed = errors.New("xlsxgraft: the book is closed: use it inside the function Open calls")

// Book is a workbook open to read, handed to the function [Open] calls: its
// sheets, and the rows of each through [Book.Sheet] and [Book.SheetAt].
//
// Usable inside that function only. When it returns, however it returns,
// the book is closed, and with it every sheet's rows; using them then gives
// [ErrClosed]. A Book may be used from several goroutines at once; each of
// its [Rows] from one at a time.
//
// Nothing here fails for what the workbook holds. A workbook that cannot be
// read lists no sheets, and a sheet the engine refuses (no such sheet, not a
// worksheet) reads no rows; the [Result] fails saying why, as it does when a
// problem stops a sheet's rows or the CLI fails. The errors a Book gives,
// but [ErrClosed], are those [Open] returns: a CLI that cannot be started,
// ctx done, a negative index.
//
// The list of the sheets (once) and each sheet's rows runs the CLI, which
// opens the file again: a file changed in between is read as it is then. So each
// reports the problems of opening it again; a diagnostic is kept as many
// times as one run gave it.
type Book struct {
	ctx  context.Context
	cli  string
	path string
	loc  *time.Location
	// temporary is a file of the book's own, removed when it closes; "" when
	// it has none.
	temporary string

	mu     sync.Mutex
	closed bool
	// runs are the reads open: each is stopped by its end, its Rows' Close
	// or the book's close.
	runs map[*readRun]struct{}
	// failed is the first thing that stopped a call short.
	failed *Diagnostic
	// raised is the first error the book gave its caller.
	raised      error
	diagnostics []Diagnostic
	// kept is how many times one run gave the diagnostic of each JSON, at
	// most.
	kept map[string]int

	// listing is held while the sheets are listed, so they are listed once.
	listing sync.Mutex
	sheets  []SheetInfo
	listed  bool
}

func newBook(ctx context.Context, cli, path string, loc *time.Location, temporary string) *Book {
	return &Book{ctx: ctx, cli: cli, path: path, loc: loc, temporary: temporary, runs: map[*readRun]struct{}{}, kept: map[string]int{}}
}

// SheetInfo is one sheet of a [Book], as [Book.Sheets] lists it.
type SheetInfo struct {
	// Index is its place in the list, from 0, as [Book.SheetAt] takes it.
	Index int
	Name  string
	Kind  SheetKind
	State SheetState
}

// SheetKind is which part a sheet is.
type SheetKind string

// The kinds of sheet. SheetKindUnknown is a sheet whose part is not known,
// or of a kind newer than this package.
const (
	Worksheet        SheetKind = "worksheet"
	Chartsheet       SheetKind = "chartsheet"
	Dialogsheet      SheetKind = "dialogsheet"
	Macrosheet       SheetKind = "macrosheet"
	SheetKindUnknown SheetKind = "unknown"
)

// SheetState is whether a sheet is shown.
type SheetState string

// The states of a sheet. SheetStateUnknown is one newer than this package.
const (
	Visible           SheetState = "visible"
	Hidden            SheetState = "hidden"
	VeryHidden        SheetState = "veryHidden"
	SheetStateUnknown SheetState = "unknown"
)

// Sheets is the sheets, in the order the workbook lists them, hidden ones
// and every kind included, listed by one run of `xlsx-graft sheets` and
// kept; none when the workbook cannot be read.
func (b *Book) Sheets() ([]SheetInfo, error) {
	b.listing.Lock()
	defer b.listing.Unlock()
	if err := b.check(); err != nil {
		return nil, err
	}
	if !b.listed {
		ran, err := runCLI(b.ctx, b.cli, "sheets", "--format", "json", "--", b.path)
		if err != nil {
			return nil, b.raise(err)
		}
		b.sheets, b.listed = b.list(ran), true
	}
	return slices.Clone(b.sheets), nil
}

// list keeps what the run of `sheets` came to, and returns the sheets it
// listed.
func (b *Book) list(ran run) []SheetInfo {
	f := &fields{}
	var sheets []SheetInfo
	var r report
	if ran.status == 0 || ran.status == 1 {
		var o object
		o, f.err = objectOf([]byte(ran.stdout))
		r = reportOf(f, o)
		listing, _ := objects(f, o, "sheets")
		for _, sheet := range listing {
			sheets = append(sheets, SheetInfo{
				Index: get[int](f, sheet, "index"),
				Name:  get[string](f, sheet, "name"),
				Kind:  known(SheetKind(get[string](f, sheet, "kind")), Worksheet, Chartsheet, Dialogsheet, Macrosheet),
				State: known(SheetState(get[string](f, sheet, "state")), Visible, Hidden, VeryHidden),
			})
		}
	}
	b.mu.Lock()
	defer b.mu.Unlock()
	switch {
	case ran.status != 0 && (ran.status != 1 || f.err != nil):
		// Exit 1 with no object is the host failing.
		b.fail(exitStatus(ran))
		sheets = nil
	case f.err != nil:
		b.fail(invalidOutput("the sheets", ran.stdout))
		sheets = nil
	default:
		b.record(r)
	}
	return sheets
}

// known is value when it is one of names, else "unknown".
func known[T ~string](value T, names ...T) T {
	if slices.Contains(names, value) {
		return value
	}
	return "unknown"
}

// Sheet is the sheet named name, letter case aside. Nothing is read until
// [Sheet.Rows]. A name no sheet has is the engine's to refuse there; to look
// for a sheet, look in [Book.Sheets]: a refused sheet fails the result.
func (b *Book) Sheet(name string) *Sheet {
	return &Sheet{book: b, name: name, index: -1}
}

// SheetAt is the sheet at index, from 0, hidden sheets and chartsheets
// counted; SheetAt(0) is the first. Nothing is read until [Sheet.Rows]; an
// index no sheet has is the engine's to refuse there. A negative index is
// an error, which its rows give.
func (b *Book) SheetAt(index int) *Sheet {
	return &Sheet{book: b, index: index, byIndex: true}
}

// Sheet is one sheet of a [Book], asked for by name or index.
type Sheet struct {
	book    *Book
	name    string
	index   int
	byIndex bool
}

// check gives ErrClosed once the book is closed.
func (b *Book) check() error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.closed {
		return ErrClosed
	}
	return nil
}

// raise keeps err, unless one is kept, for [Open] to return, and returns it.
func (b *Book) raise(err error) error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.raised == nil {
		b.raised = err
	}
	return err
}

// record keeps a run's report: its refusal, unless a failure is kept, and
// its diagnostics but those an earlier run gave as often. b.mu is held.
func (b *Book) record(r report) {
	if r.refused != nil {
		b.fail(*r.refused)
	}
	counted := map[string]int{}
	for i, key := range r.keys {
		counted[key]++
		if counted[key] > b.kept[key] {
			b.kept[key] = counted[key]
			b.diagnostics = append(b.diagnostics, r.diagnostics[i])
		}
	}
}

// fail keeps d, unless a failure is kept: the first is the one kept. b.mu is
// held.
func (b *Book) fail(d Diagnostic) {
	if b.failed == nil {
		b.failed = &d
	}
}

// close stops every read still open and removes the book's own file.
func (b *Book) close() {
	b.mu.Lock()
	b.closed = true
	runs := b.runs
	b.runs = nil
	b.mu.Unlock()
	for run := range runs {
		run.stop()
	}
	if b.temporary != "" {
		_ = os.Remove(b.temporary)
	}
}

// resultOf is value on success: no failure, and no diagnostic an error.
func resultOf[T any](b *Book, value T) Result[T] {
	b.mu.Lock()
	defer b.mu.Unlock()
	errs := slices.ContainsFunc(b.diagnostics, func(d Diagnostic) bool { return d.Severity == "error" })
	if b.failed != nil || errs {
		return Result[T]{Diagnostics: slices.Clone(b.diagnostics), Error: b.failed}
	}
	return Result[T]{Success: true, Value: value, Diagnostics: slices.Clone(b.diagnostics)}
}
