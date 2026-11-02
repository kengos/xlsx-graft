package xlsxgraft

import (
	"context"
	"testing"
	"time"
)

// What a book does when its close comes between two steps of a read, as it
// does when the caller reads on from another goroutine after the function
// Open called returned.

func TestRowsStartedAsTheBookClosesAreStopped(t *testing.T) {
	b := newBook(context.Background(), fake(t, `while printf '%s\n' x; do :; done`), "book.xlsx", time.UTC, "")
	b.close()
	started := time.Now()

	if run, err := b.openRows(b.SheetAt(0)); run != nil || err != ErrClosed || time.Since(started) > 30*time.Second {
		t.Errorf("openRows = %v, %v", run, err)
	}
}

func TestRowsEndedAfterTheBookClosedAreNotKept(t *testing.T) {
	b := newBook(context.Background(), fake(t, lines(endLine)), "book.xlsx", time.UTC, "")
	run, err := b.openRows(b.SheetAt(0))
	if err != nil {
		t.Fatal(err)
	}
	b.close()

	if err := b.endRows(run); err != ErrClosed {
		t.Errorf("endRows = %v", err)
	}
	if got := resultOf(b, 1); !got.Success {
		t.Errorf("result = %+v", got)
	}
}
