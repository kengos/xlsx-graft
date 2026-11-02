package io.github.kengos.xlsxgraft;

import com.sun.jna.ptr.PointerByReference;
import java.util.Iterator;
import java.util.NoSuchElementException;

/**
 * A sheet's rows, from {@link Sheet#rows()}, read one at a time as they are iterated, once. Rows
 * closed end there: iterating them after gives no more.
 *
 * <pre>{@code
 * try (Rows rows = book.sheet("Data").rows()) {
 *   for (Row row : rows) {
 *     System.out.println(row.number() + ": " + row.values());
 *   }
 * }
 * }</pre>
 *
 * <p>What the engine keeps of the sheet is freed when the last row has been read, when the rows are
 * closed, or when the book is, whichever comes first. A row asked for after the book is closed
 * throws {@link IllegalStateException}.
 */
public final class Rows implements Iterable<Row>, AutoCloseable {

  // Held so that the rows are read through, and closed with, their book.
  private final Book book;

  // The rows handle's slot, null for a sheet the engine refused.
  private final PointerByReference slot;

  private boolean iterated;

  Rows(Book book, PointerByReference slot) {
    this.book = book;
    this.slot = slot;
  }

  /**
   * The rows, in order.
   *
   * @return an iterator over the rows; {@code remove} is not supported
   * @throws IllegalStateException the rows were iterated already: call {@link Sheet#rows()} again
   *     to read the sheet again
   */
  @Override
  public synchronized Iterator<Row> iterator() {
    if (iterated) {
      throw new IllegalStateException("rows are iterated once: call Sheet.rows() again");
    }
    iterated = true;
    return new Iterator<Row>() {

      private Row next;
      private boolean ended = slot == null;

      @Override
      public boolean hasNext() {
        if (next == null && !ended) {
          next = book.nextRow(slot);
          if (next == null) {
            ended = true;
            book.closeRows(slot);
          }
        }
        return next != null;
      }

      @Override
      public Row next() {
        if (!hasNext()) {
          throw new NoSuchElementException("the sheet has no more rows");
        }
        Row row = next;
        next = null;
        return row;
      }
    };
  }

  /** Frees what the engine keeps of the sheet, unless the read's end or the book's close did. */
  @Override
  public void close() {
    if (slot != null) {
      book.closeRows(slot);
    }
  }
}
