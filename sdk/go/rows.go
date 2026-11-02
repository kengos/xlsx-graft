package xlsxgraft

import (
	"errors"
	"fmt"
	"strconv"
	"strings"
	"time"
	"unicode/utf8"
)

// Rows is the rows a sheet writes, in order, pulled one at a time as a
// [bufio.Scanner]'s lines are:
//
//	rows := book.Sheet("Data").Rows()
//	defer rows.Close()
//	for rows.Next() {
//		row := rows.Row()
//		...
//	}
//	if err := rows.Err(); err != nil {
//		return 0, err
//	}
//
// The first Next starts a run of `xlsx-graft read`, which reads the sheet
// from its start: no more of the sheet is held than the row given and what
// the pipe holds of the rows after it.
//
// Reading stops at the first problem in the sheet, the rows before it having
// come, and gives nothing when the engine refuses the sheet; either way Next
// returns false, Err is nil, and the [Result] fails saying why. Err is an
// error [Open] returns too (a CLI that cannot be started, ctx done, which
// kills the CLI, or a negative index), or [ErrClosed], which it does not.
type Rows struct {
	sheet *Sheet
	run   *readRun
	row   Row
	err   error
	done  bool
}

// Rows is the sheet's rows, read when [Rows.Next] is first called.
func (s *Sheet) Rows() *Rows {
	return &Rows{sheet: s}
}

// Next reads the next row, which [Rows.Row] then gives, and reports whether
// there was one.
func (r *Rows) Next() bool {
	if r.done {
		return false
	}
	b := r.sheet.book
	if err := b.check(); err != nil {
		return r.end(err)
	}
	if r.run == nil {
		run, err := b.openRows(r.sheet)
		if err != nil || run == nil {
			return r.end(err)
		}
		r.run = run
	}
	row, stop := r.run.next(b.loc)
	switch {
	case row != nil:
		r.row = *row
		return true
	case stop != nil:
		b.stopRows(r.run, *stop)
		return r.end(nil)
	default:
		return r.end(b.endRows(r.run))
	}
}

func (r *Rows) end(err error) bool {
	r.err, r.done = err, true
	return false
}

// Row is the row the last [Rows.Next] read.
func (r *Rows) Row() Row {
	return r.row
}

// Err is the error that ended the rows, nil when they ended as the sheet
// did, a problem in it, or the engine's refusal (which the [Result] says).
func (r *Rows) Err() error {
	return r.err
}

// Close stops the rows: when the CLI is still writing, it closes its stdout
// and waits for it to stop, at the next output it flushes. That is no
// failure, and the run reports no diagnostics. Rows neither read to the end
// nor closed run until the book closes.
func (r *Rows) Close() {
	r.done = true
	if r.run != nil {
		r.sheet.book.stopRows(r.run, Diagnostic{})
	}
}

// openRows starts the run reading s; or nil when this package refuses the
// name, which the result says.
func (b *Book) openRows(s *Sheet) (*readRun, error) {
	arg := "--sheet=" + s.name
	switch {
	case s.byIndex && s.index < 0:
		return nil, b.raise(fmt.Errorf("xlsxgraft: a sheet index is from 0, not %d", s.index))
	case s.byIndex:
		arg = "--sheet-index=" + strconv.Itoa(s.index)
	case !utf8.ValidString(s.name) || strings.ContainsRune(s.name, 0):
		b.mu.Lock()
		defer b.mu.Unlock()
		b.fail(problem("invalid_arg", "a sheet name is UTF-8 text with no NUL"))
		return nil, nil
	}
	process, err := startCLI(b.ctx, b.cli, "read", "--format", "json", arg, "--", b.path)
	if err != nil {
		return nil, b.raise(err)
	}
	run := &readRun{process: process}
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.closed {
		// The book closed while the CLI started.
		run.stop()
		return nil, ErrClosed
	}
	b.runs[run] = struct{}{}
	return run, nil
}

// stopRows stops run, unless its end or the book's close did, keeping why:
// a zero Diagnostic is no failure.
func (b *Book) stopRows(run *readRun, why Diagnostic) {
	b.mu.Lock()
	_, open := b.runs[run]
	delete(b.runs, run)
	if why.Code != "" {
		b.fail(why)
	}
	b.mu.Unlock()
	if open {
		run.stop()
	}
}

// endRows keeps what run came to, once its stdout ended, unless the book's
// close stopped it; the error is the context's, when it killed the CLI.
func (b *Book) endRows(run *readRun) error {
	b.mu.Lock()
	_, open := b.runs[run]
	delete(b.runs, run)
	b.mu.Unlock()
	if !open {
		return ErrClosed
	}
	r, failed, err := run.outcome()
	if err != nil {
		return b.raise(err)
	}
	b.mu.Lock()
	defer b.mu.Unlock()
	if failed != nil {
		b.fail(*failed)
	} else {
		b.record(r)
	}
	return nil
}

// readRun is one run of `xlsx-graft read --format json`: its rows a line at a
// time, then what it came to, decided as docs/sdk.md § Reading rows says.
type readRun struct {
	process *stream
	// end is the end line's report, once read.
	end *report
	// last is the line that broke the contract, or the last line read.
	last    []byte
	invalid bool
}

// next is the next row; or why this package cannot give it (the run is then
// to be stopped); or neither once stdout ended, outcome then saying how.
func (r *readRun) next(loc *time.Location) (*Row, *Diagnostic) {
	for {
		line, more := r.process.line()
		if !more {
			return nil, nil
		}
		r.last = line
		f := &fields{}
		row, stop := r.take(f, line, loc)
		if f.err != nil {
			r.invalid = true
			// What the run comes to is read from its status once it ends:
			// a run that failed may have printed anything, so the rest is
			// read rather than the CLI stopped.
			r.process.drain()
		} else if row != nil || stop != nil {
			return row, stop
		}
	}
}

// take reads line: a row, or why it cannot be given, or the end line, kept.
func (r *readRun) take(f *fields, line []byte, loc *time.Location) (*Row, *Diagnostic) {
	if r.end != nil {
		f.fail(errors.New("a line after the end line"))
		return nil, nil
	}
	var o object
	o, f.err = objectOf(line)
	if _, isRow := o["row"]; !isRow {
		end := reportOf(f, o)
		get[int](f, o, "rows")
		has(f, o, "sheet")
		if f.err == nil {
			r.end = &end
		}
		return nil, nil
	}
	number := get[int](f, o, "row")
	cells, _ := objects(f, o, "cells")
	values, stop := cellValues(f, cells, loc)
	if stop != nil {
		d := problem(stop.Code, fmt.Sprintf("row %d: %s", number, stop.Message))
		return nil, &d
	}
	return &Row{Number: number, Values: values}, nil
}

// outcome is what the run came to, once next gave neither a row nor a stop:
// the end line's report, or why there is none. The error is the context's,
// when it killed the CLI.
func (r *readRun) outcome() (report, *Diagnostic, error) {
	ran, err := r.process.finish()
	switch {
	case err != nil:
		return report{}, nil, err
	case (ran.status == 0 || ran.status == 1) && !r.invalid && r.end != nil:
		return *r.end, nil, nil
	case ran.status != 0:
		d := exitStatus(ran)
		return report{}, &d, nil
	default:
		d := invalidOutput("the end line last", string(r.last))
		return report{}, &d, nil
	}
}

// stop closes stdout, the caller having stopped taking rows, and waits for
// the CLI to end: what it comes to is not a failure, whatever it is.
func (r *readRun) stop() {
	_, _ = r.process.finish()
}
