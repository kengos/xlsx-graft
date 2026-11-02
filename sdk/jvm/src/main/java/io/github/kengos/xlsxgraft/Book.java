// A book and its rows share state the library does not lock, so every call
// into it, closes included, is made under the book's own lock, and what it
// lends is copied out before the lock is let go.
//
// The JVM raises no exception between two plain lines but the deprecated
// Thread.stop's (gone since Java 20) and a VirtualMachineError (out of
// memory, out of stack); docs/sdk.md's rules for one hold against those and
// an exception of the package's own: a handle's slot exists before the call
// that fills it, and a handle leaves the book before the call that frees it,
// so such an exception leaks a handle at worst, never frees one twice; the
// result is a close's that ran whole. There is no finalizer or Cleaner: they
// run on a thread of their own, and the book is closed by its
// try-with-resources block.

package io.github.kengos.xlsxgraft;

import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * A workbook open to read, from {@link XlsxReader}: its sheets, and the rows of each through {@link
 * #sheet(String)}.
 *
 * <p>Read it in a try-with-resources block, which closes it however the block ends, and with it
 * every sheet's rows; using it after throws {@link IllegalStateException}. Then {@link #result()}
 * says how the reading went.
 *
 * <p>Nothing here leaves the block early. A file that cannot be opened lists no sheets, and a sheet
 * the engine refuses (no such sheet, an unreadable workbook) reads no rows; the {@link #result()}
 * fails, saying why, as it does when a problem stops a sheet's rows or the host fails. A panic in
 * the engine ends every sheet of the book: none reads a row after it, and the book lists no sheets.
 * An exception the block throws is the caller's: it propagates, the book closed first.
 *
 * <p>One thread at a time or several: every call takes the book's lock.
 */
public final class Book implements AutoCloseable {

  private final Capi capi;
  private final Object lock = new Object();

  // The book handle's slot, written by the open.
  private final PointerByReference handle = new PointerByReference();

  // The rows handles' slots open, each freed by its sheet's read or at the end.
  private final List<PointerByReference> rows = new ArrayList<>();

  private boolean closed;

  // Why the package stopped short where the engine keeps no failure.
  private Diagnostic failure;
  private Result<Void> result;

  /** Opens the book: {@code opening} writes its handle into the slot it is given. */
  Book(Capi capi, Opening opening) {
    this.capi = capi;
    try {
      int status = opening.open(capi, handle);
      // Read only on OK, as the header says: a panic may have written half
      // of it.
      if (status != StatusDiagnostic.OK) {
        handle.setValue(null);
      }
      failed(status);
    } catch (RuntimeException | Error error) {
      try {
        close();
      } catch (RuntimeException | Error closing) {
        error.addSuppressed(closing);
      }
      throw error;
    }
  }

  /** Opens a book through {@code capi}, writing its handle into {@code book}. */
  interface Opening {
    int open(Capi capi, PointerByReference book);
  }

  /**
   * How the reading went, once the book is closed (after its try-with-resources block). Its error
   * is the first failure; the engine's words for it when the engine kept it too. It has no value:
   * keep what you read in a variable outside the block.
   *
   * <p>A close an exception cut short read no diagnostics: its result fails with {@code
   * interrupted}, never a success.
   *
   * @return the result
   * @throws IllegalStateException the book is not closed yet
   */
  public Result<Void> result() {
    // Locked: a close under way on another thread has not set it yet.
    synchronized (lock) {
      if (!closed) {
        throw new IllegalStateException(
            "the result is known once the book is closed, after its try-with-resources block");
      }
      return result != null
          ? result
          : Result.<Void>failure(
              Collections.<Diagnostic>emptyList(),
              StatusDiagnostic.error(
                  "interrupted",
                  "an exception cut the book's close short, before it read the diagnostics"));
    }
  }

  /**
   * The sheets, in the order the workbook lists them, hidden ones included; none when the workbook
   * cannot be read.
   *
   * @return each sheet's index, name, kind and state, which cannot be changed
   * @throws IllegalStateException the book is closed
   */
  public List<SheetInfo> sheets() {
    List<SheetInfo> sheets = new ArrayList<>();
    synchronized (lock) {
      checkOpen();
      failed(capi.bookSheets(handle.getValue(), sheets));
    }
    return Collections.unmodifiableList(sheets);
  }

  /**
   * The first sheet.
   *
   * @return the sheet, to read its rows
   * @throws IllegalStateException the book is closed
   */
  public Sheet sheet() {
    return sheet(0);
  }

  /**
   * The sheet named {@code name}, letter case aside. Nothing is read until {@link Sheet#rows()}. A
   * name no sheet has is the engine's to refuse there; to look for a sheet, look in {@link
   * #sheets()}: a refused sheet fails the {@link #result()}.
   *
   * @param name the sheet's name
   * @return the sheet, to read its rows
   * @throws NullPointerException {@code name} is null
   * @throws IllegalStateException the book is closed
   */
  public Sheet sheet(String name) {
    Objects.requireNonNull(name, "name");
    synchronized (lock) {
      checkOpen();
    }
    return new Sheet(this, name, 0);
  }

  /**
   * The sheet at {@code index}, from 0, hidden sheets and chartsheets counted. Nothing is read
   * until {@link Sheet#rows()}. An index no sheet has is the engine's to refuse there; to look for
   * a sheet, look in {@link #sheets()}: a refused sheet fails the {@link #result()}.
   *
   * @param index from 0
   * @return the sheet, to read its rows
   * @throws IllegalArgumentException {@code index} is below 0
   * @throws IllegalStateException the book is closed
   */
  public Sheet sheet(int index) {
    if (index < 0) {
      throw new IllegalArgumentException("a sheet's index is from 0, not " + index);
    }
    synchronized (lock) {
      checkOpen();
    }
    return new Sheet(this, null, index);
  }

  /**
   * Reads the diagnostics, then closes the book, which closes its rows, and frees what is left of
   * each rows handle. Once: a second call does nothing. The try-with-resources block calls it.
   */
  @Override
  public void close() {
    synchronized (lock) {
      if (closed) {
        return;
      }
      closed = true;
      EngineDiagnostics.Report report;
      try {
        report = report();
      } finally {
        free();
      }
      // The last statement: a close an exception cuts short leaves no
      // result, which result() reads as `interrupted`.
      result = outcome(report);
    }
  }

  // What Sheet and Rows call.

  /** The rows handle's slot, null when the sheet was refused. */
  PointerByReference openRows(String name, int index) {
    synchronized (lock) {
      checkOpen();
      // Listed before the call, so an exception after it still finds the
      // handle to free; a NULL one frees nothing.
      PointerByReference slot = new PointerByReference();
      rows.add(slot);
      int status;
      if (name == null) {
        status = capi.rowsOpen(handle.getValue(), index, slot);
      } else {
        // A name that is not Unicode is refused as the engine refuses one
        // that is not UTF-8.
        byte[] encoded = Capi.utf8(name);
        status =
            encoded == null
                ? StatusDiagnostic.INVALID_ARG
                : capi.rowsOpenName(handle.getValue(), encoded, slot);
      }
      failed(status);
      if (status == StatusDiagnostic.OK) {
        return slot;
      }
      rows.remove(slot);
      return null;
    }
  }

  /**
   * The next row; null at the end of the sheet, or when reading it stopped short, which the {@link
   * #result()} says.
   */
  Row nextRow(PointerByReference slot) {
    synchronized (lock) {
      checkOpen();
      // Freed already, by its rows' close: the end of them.
      if (!rows.contains(slot)) {
        return null;
      }
      Capi.Next next = capi.rowsNext(slot.getValue());
      failed(next.status);
      if (next.status != StatusDiagnostic.OK || next.number == 0) {
        return null;
      }
      try {
        return new Row(Math.toIntExact(next.number), CellValues.call(next.values, next.count));
      } catch (CellValues.UnknownValueException refused) {
        failWith(
            StatusDiagnostic.error(
                "unknown_value", "row " + next.number + ": " + refused.getMessage()));
        return null;
      }
    }
  }

  /** Frees a rows handle {@link #openRows} made, unless the book's close did. */
  void closeRows(PointerByReference slot) {
    synchronized (lock) {
      if (rows.remove(slot)) {
        failed(capi.rowsClose(slot.getValue()));
        slot.setValue(null);
      }
    }
  }

  /**
   * Closes the book, which closes its rows, and frees what is left of each rows handle. Each leaves
   * the book before its call frees it.
   */
  private void free() {
    failed(capi.bookClose(handle.getValue()));
    handle.setValue(null);
    while (!rows.isEmpty()) {
      Pointer left = rows.remove(rows.size() - 1).getValue();
      failed(capi.rowsClose(left));
    }
  }

  private void checkOpen() {
    if (closed) {
      throw new IllegalStateException(
          "the book is closed: use it inside its try-with-resources block");
    }
  }

  /**
   * Keeps {@code status} as the book's failure unless it is OK or one is kept, with the host's
   * reason for an IO one, which Capi read as the call returned.
   */
  private void failed(int status) {
    if (status == StatusDiagnostic.OK) {
      return;
    }
    String message = status == StatusDiagnostic.IO ? capi.reason() : null;
    failWith(StatusDiagnostic.of(status, message));
  }

  private void failWith(Diagnostic diagnostic) {
    if (failure == null) {
      failure = diagnostic;
    }
  }

  /**
   * The book's diagnostics; none when they cannot be read. A book that did not open has none, its
   * handle NULL, which the library refuses.
   */
  private EngineDiagnostics.Report report() {
    Capi.Lent lent = capi.bookDiagnostics(handle.getValue());
    if (lent.status != StatusDiagnostic.OK) {
      failed(lent.status);
      return EngineDiagnostics.UNREAD;
    }
    try {
      return EngineDiagnostics.call(lent.bytes);
    } catch (Json.UnreadableException error) {
      failWith(
          StatusDiagnostic.error(
              "diagnostics_unreadable",
              "the engine's diagnostics could not be read: " + error.getMessage()));
      return EngineDiagnostics.UNREAD;
    }
  }

  /**
   * The package's first failure, or the engine's: the engine's words for it when it kept the same.
   */
  private Result<Void> outcome(EngineDiagnostics.Report report) {
    Diagnostic error = report.error;
    if (failure != null && (error == null || !failure.code().equals(error.code()))) {
      error = failure;
    }
    return error != null || report.hasErrors
        ? Result.<Void>failure(report.diagnostics, error)
        : Result.<Void>ok(null, report.diagnostics);
  }
}
