package xlsxgraft

import (
	"context"
	"errors"
	"fmt"
	"os"
	"strings"
)

// Open opens the workbook at path, hands it to read as a [Book], and closes
// it when read returns, however it returns.
//
//	result, err := xlsxgraft.Open(ctx, "book.xlsx", func(book *xlsxgraft.Book) (int, error) {
//		rows := book.Sheet("Data").Rows()
//		defer rows.Close()
//		count := 0
//		for rows.Next() {
//			fmt.Println(rows.Row().Number, rows.Row().Values)
//			count++
//		}
//		return count, rows.Err()
//	}, xlsxgraft.WithLocation(tokyo))
//
// The result's value is what read returned, on success. It fails with Error
// set when the workbook or a sheet asked for is refused, or the CLI fails (a
// file that cannot be opened is one: `exit_status`, quoting why); and
// without, when a problem stopped a sheet's rows, its diagnostic an error. A
// path holding a NUL is `invalid_arg`, and read is not called.
//
// The error is read's own, when it returns one, or else the first the book
// gave read (as a missed exception would propagate): a [*CLINotFoundError]
// when no CLI is found or it cannot be started, ctx's error when ctx ended a
// run of the CLI (killing it), or a negative sheet index. The result is then
// the zero Result. A panic in read propagates, the book closed first.
func Open[T any](ctx context.Context, path string, read func(*Book) (T, error), opts ...Option) (Result[T], error) {
	if strings.ContainsRune(path, 0) {
		return refused[T](problem("invalid_arg", "a path holds no NUL")), nil
	}
	o := apply(opts)
	cli, err := findCLI(o.cli, os.Getenv)
	if err != nil {
		return Result[T]{}, err
	}
	return reading(newBook(ctx, cli, path, o.location, ""), read)
}

// OpenBytes is as [Open], the workbook being data, written to a temporary
// file of the book's own (the CLI reads a path) that only its user may read,
// removed when the book closes. A file that cannot be written is `io`, and
// read is not called.
func OpenBytes[T any](ctx context.Context, data []byte, read func(*Book) (T, error), opts ...Option) (Result[T], error) {
	o := apply(opts)
	cli, err := findCLI(o.cli, os.Getenv)
	if err != nil {
		return Result[T]{}, err
	}
	path, err := writeTemporary(os.TempDir(), data)
	if err != nil {
		return refused[T](problem("io", fmt.Sprintf("the workbook could not be written to a temporary file (%v)", err))), nil
	}
	return reading(newBook(ctx, cli, path, o.location, path), read)
}

func reading[T any](b *Book, read func(*Book) (T, error)) (Result[T], error) {
	value, err := func() (T, error) {
		defer b.close()
		return read(b)
	}()
	if err == nil {
		b.mu.Lock()
		err = b.raised
		b.mu.Unlock()
	}
	if err != nil {
		return Result[T]{}, err
	}
	return resultOf(b, value), nil
}

// refused is a failure for d, read not called.
func refused[T any](d Diagnostic) Result[T] {
	return Result[T]{Error: &d}
}

// writeTemporary writes data to a new file in dir, which only this user may
// read (os.CreateTemp makes it 0600), and returns its path.
func writeTemporary(dir string, data []byte) (string, error) {
	path := ""
	file, err := os.CreateTemp(dir, "xlsx-graft-*.xlsx")
	if err == nil {
		path = file.Name()
		_, err = file.Write(data)
		err = errors.Join(err, file.Close())
	}
	if err != nil {
		// path is "" when no file was made, and removing it does nothing.
		_ = os.Remove(path)
		return "", err
	}
	return path, nil
}
