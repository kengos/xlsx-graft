// What the engine cannot be made to do from a workbook: panic, fail the host
// while lending the diagnostics, answer a status or lend a value newer than
// the package. Each test reads a real workbook through a Capi of which one
// call or more is replaced.

package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.jna.Memory;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.Iterator;
import java.util.List;
import java.util.function.Function;
import java.util.function.Supplier;
import java.util.function.ToIntFunction;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class BookTest {

  private static final byte[] BYTES =
      Workbook.xlsx(Workbook.worksheet("Data", "<row r=\"1\"/><row r=\"2\"/>"));

  @Test
  void aPanicReadingARowFailsWithPanicReadingNoDiagnostics() {
    // The engine refuses every call after a panic.
    FakeCapi capi = new FakeCapi();
    capi.rowsNextWith = rows -> new Capi.Next(2, 0, null, 0);
    capi.bookDiagnosticsWith = book -> new Capi.Lent(2, new byte[0]);

    Read read = read(capi);

    assertTrue(read.rows.isEmpty());
    assertFalse(read.result.success());
    assertTrue(read.result.diagnostics().isEmpty());
    assertEquals("panic", read.result.error().code());
  }

  @Test
  void aStatusThePackageDoesNotKnowFailsWithUnknownStatus() {
    FakeCapi capi = new FakeCapi();
    capi.rowsOpenWith = (book, slot) -> 42;

    Diagnostic error = read(capi).result.error();

    assertEquals("unknown_status", error.code());
    assertTrue(error.message().contains("42"), error.message());
  }

  @Test
  void aValueOfAKindNewerThanThePackageStopsTheSheetWithUnknownValue() {
    Memory value = new Memory(CellValues.SIZE);
    value.clear();
    value.setInt(0, 9);
    FakeCapi capi = new FakeCapi();
    capi.rowsNextWith = rows -> new Capi.Next(0, 1, value, 1);
    Book book = open(capi);
    List<Integer> rows;
    try (Book open = book) {
      rows = XlsxReaderTest.numbers(open.sheet().rows());

      // A refused sheet after it does not replace it.
      assertTrue(XlsxReaderTest.numbers(open.sheet("Missing").rows()).isEmpty());
    }

    assertTrue(rows.isEmpty());
    assertEquals(
        StatusDiagnostic.error(
            "unknown_value",
            "row 1: the engine returned a value of kind 9, which this package does not know"),
        book.result().error());

    // A kind past what an int holds is not taken for one below 0.
    value.setInt(0, -1);
    assertTrue(
        read(capi).result.error().message().contains("kind 4294967295"),
        read(capi).result.error().message());
  }

  @Test
  void diagnosticsThatCannotBeLentFailWithIoSayingWhy() {
    FakeCapi capi = new FakeCapi();
    capi.bookDiagnosticsWith = book -> new Capi.Lent(3, new byte[0]);
    capi.reasonWith = () -> "the disk is full";

    Read read = read(capi);

    assertEquals(Arrays.asList(1, 2), read.rows);
    assertEquals(StatusDiagnostic.error("io", "the disk is full"), read.result.error());
  }

  @Test
  void emptyDiagnosticsFailWithDiagnosticsUnreadable() {
    FakeCapi capi = new FakeCapi();
    capi.bookDiagnosticsWith = book -> new Capi.Lent(0, new byte[0]);

    assertEquals(
        StatusDiagnostic.error(
            "diagnostics_unreadable",
            "the engine's diagnostics could not be read: the JSON ends early"),
        read(capi).result.error());
  }

  @ParameterizedTest
  @ValueSource(
      strings = {
        "{}",
        "[]",
        "{\"has_errors\": false, \"error\": null, \"diagnostics\": {}}",
        "{\"has_errors\": false, \"error\": {}, \"diagnostics\": []}",
        "{\"has_errors\": false, \"error\": {\"code\": null, \"message\": \"m\"}, \"diagnostics\": []}",
        "{\"has_errors\": 0, \"error\": null, \"diagnostics\": []}",
        "{\"has_errors\": false, \"error\": null, \"diagnostics\": [1]}",
      })
  void diagnosticsNotOfTheShapeFailWithDiagnosticsUnreadable(String json) {
    FakeCapi capi = new FakeCapi();
    capi.bookDiagnosticsWith = book -> new Capi.Lent(0, json.getBytes(StandardCharsets.UTF_8));

    Diagnostic error = read(capi).result.error();

    assertEquals("diagnostics_unreadable", error.code());
    assertTrue(
        error.message().startsWith("the engine's diagnostics could not be read: "),
        error.message());
  }

  @Test
  void aPanicFreeingARowsHandleFailsWithPanic() {
    FakeCapi capi = new FakeCapi();
    capi.rowsCloseWith = rows -> 2;

    Read read = read(capi);

    assertEquals(Arrays.asList(1, 2), read.rows);
    assertEquals("panic", read.result.error().code());
  }

  @Test
  void aSheetOfAKindOrStateNewerThanThePackageIsListedAsUnknown() {
    assertEquals(SheetKind.UNKNOWN, SheetKind.of(7));
    assertEquals(SheetKind.UNKNOWN, SheetKind.of(-1));
    assertEquals(SheetKind.MACROSHEET, SheetKind.of(3));
    assertEquals(SheetState.UNKNOWN, SheetState.of(3));
    assertEquals(SheetState.UNKNOWN, SheetState.of(-1));
    assertEquals(SheetState.HIDDEN, SheetState.of(1));
  }

  @Test
  void anExceptionAfterTheOpenClosesTheHandleItMade() {
    FakeCapi capi = new FakeCapi();
    Pointer[] handle = new Pointer[1];

    assertThrows(
        StackOverflowError.class,
        () ->
            new Book(
                capi,
                (c, book) -> {
                  c.bookOpenBytes(BYTES, book);
                  handle[0] = book.getValue();
                  throw new StackOverflowError();
                }));

    assertNotNull(handle[0]);
    assertEquals(Collections.singletonList(handle[0]), capi.bookCloses);
  }

  @Test
  void aRuntimeExceptionAfterTheOpenClosesTheBookToo() {
    FakeCapi capi = new FakeCapi();

    assertThrows(
        IllegalStateException.class,
        () ->
            new Book(
                capi,
                (c, book) -> {
                  throw new IllegalStateException();
                }));

    assertEquals(1, capi.bookCloses.size());
  }

  @Test
  void anExceptionClosingAfterTheOpenFailedIsKeptSuppressed() {
    FakeCapi capi = new FakeCapi();
    StackOverflowError closing = new StackOverflowError();
    capi.bookCloseWith =
        book -> {
          throw closing;
        };

    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () ->
                new Book(
                    capi,
                    (c, book) -> {
                      throw new IllegalStateException();
                    }));

    assertEquals(Collections.singletonList(closing), Arrays.asList(error.getSuppressed()));
  }

  @ParameterizedTest
  @ValueSource(booleans = {true, false})
  void aCloseAnExceptionCutsShortFailsWithInterrupted(boolean inDiagnostics) {
    FakeCapi capi = new FakeCapi();
    if (inDiagnostics) {
      capi.bookDiagnosticsWith =
          book -> {
            throw new StackOverflowError();
          };
    } else {
      capi.bookCloseWith =
          book -> {
            throw new StackOverflowError();
          };
    }

    Book book = open(capi);
    assertTrue(book.sheet().rows().iterator().hasNext());

    assertThrows(StackOverflowError.class, book::close);
    // Closed all the same: a second close does nothing.
    book.close();
    assertFalse(book.result().success());
    assertTrue(book.result().diagnostics().isEmpty());
    assertEquals("interrupted", book.result().error().code());
    assertEquals(inDiagnostics ? 1 : 0, capi.bookCloses.size());
  }

  @Test
  void anExceptionAfterARowsOpenLeavesTheHandleForTheBookToFree() {
    FakeCapi capi = new FakeCapi();
    Pointer[] handle = new Pointer[1];
    capi.rowsOpenWith =
        (book, slot) -> {
          capi.real.rowsOpen(book, 0, slot);
          handle[0] = slot.getValue();
          throw new StackOverflowError();
        };

    try (Book book = open(capi)) {
      assertThrows(StackOverflowError.class, () -> book.sheet().rows());
    }

    assertNotNull(handle[0]);
    assertEquals(Collections.singletonList(handle[0]), capi.rowsCloses);
  }

  @Test
  void rowsClosedMidwayAreFreedOnce() {
    FakeCapi capi = new FakeCapi();
    Book book = open(capi);
    try (Book open = book) {
      try (Rows rows = open.sheet().rows()) {
        assertEquals(1, rows.iterator().next().number());
      }
      assertEquals(1, capi.rowsCloses.size());
    }

    assertEquals(1, capi.rowsCloses.size());
    assertTrue(book.result().success());
  }

  @Test
  void rowsClosedInTheirLoopEndThereAndTheResultStaysASuccess() {
    FakeCapi capi = new FakeCapi();
    Book book = open(capi);
    List<Integer> read = new ArrayList<>();
    try (Book open = book) {
      Rows rows = open.sheet().rows();
      for (Row row : rows) {
        read.add(row.number());
        rows.close();
      }
    }

    assertEquals(Collections.singletonList(1), read);
    assertEquals(1, capi.rowsCloses.size());
    assertTrue(book.result().success());
  }

  @Test
  void rowsReadToTheEndAreFreedThenAndNotAgainByTheirClose() {
    FakeCapi capi = new FakeCapi();
    try (Book book = open(capi);
        Rows rows = book.sheet().rows()) {
      assertEquals(Arrays.asList(1, 2), XlsxReaderTest.numbers(rows));
      assertEquals(1, capi.rowsCloses.size());
    }

    assertEquals(1, capi.rowsCloses.size());
  }

  @Test
  void rowsLeftOpenAreFreedByTheBooksClose() {
    FakeCapi capi = new FakeCapi();
    Book book = open(capi);
    Rows rows;
    try (Book open = book) {
      rows = open.sheet().rows();
      Iterator<Row> started = rows.iterator();
      assertTrue(started.hasNext());
      assertTrue(capi.rowsCloses.isEmpty());
    }

    assertEquals(1, capi.rowsCloses.size());
    assertTrue(book.result().success());

    // Their own close after frees nothing again.
    rows.close();
    assertEquals(1, capi.rowsCloses.size());
  }

  @Test
  void theRowsOfARefusedSheetCloseFreeingNothing() {
    FakeCapi capi = new FakeCapi();
    try (Book book = open(capi)) {
      book.sheet("Missing").rows().close();
    }

    assertTrue(capi.rowsCloses.isEmpty());
  }

  private static Book open(Capi capi) {
    return new Book(capi, (c, book) -> c.bookOpenBytes(BYTES, book));
  }

  // Reads the first sheet: the result and the rows' numbers.
  private static Read read(Capi capi) {
    Book book = open(capi);
    List<Integer> rows;
    try (Book open = book) {
      rows = XlsxReaderTest.numbers(open.sheet().rows());
    }
    return new Read(book.result(), rows);
  }

  private static final class Read {

    final Result<Void> result;
    final List<Integer> rows;

    Read(Result<Void> result, List<Integer> rows) {
      this.result = result;
      this.rows = rows;
    }
  }

  /** A slot-writing call a test replaces. */
  interface RowsOpening {
    int open(Pointer book, PointerByReference slot);
  }

  /** A Capi over the engine library whose calls a test may replace, counting the closes. */
  private static final class FakeCapi extends Capi {

    // The same library, calls not replaced.
    final Capi real = new Capi(library());

    Function<Pointer, Capi.Next> rowsNextWith;
    RowsOpening rowsOpenWith;
    ToIntFunction<Pointer> rowsCloseWith;
    Function<Pointer, Capi.Lent> bookDiagnosticsWith;
    ToIntFunction<Pointer> bookCloseWith;
    Supplier<String> reasonWith;

    final List<Pointer> bookCloses = new ArrayList<>();
    final List<Pointer> rowsCloses = new ArrayList<>();

    FakeCapi() {
      super(library());
    }

    private static Library library() {
      return Library.open(Repo.ENGINE_LIBRARY, Collections.<String, String>emptyMap());
    }

    @Override
    Capi.Next rowsNext(Pointer rows) {
      return rowsNextWith == null ? super.rowsNext(rows) : rowsNextWith.apply(rows);
    }

    @Override
    int rowsOpen(Pointer book, int index, PointerByReference rows) {
      return rowsOpenWith == null
          ? super.rowsOpen(book, index, rows)
          : rowsOpenWith.open(book, rows);
    }

    @Override
    int rowsClose(Pointer rows) {
      rowsCloses.add(rows);
      return rowsCloseWith == null ? super.rowsClose(rows) : rowsCloseWith.applyAsInt(rows);
    }

    @Override
    Capi.Lent bookDiagnostics(Pointer book) {
      return bookDiagnosticsWith == null
          ? super.bookDiagnostics(book)
          : bookDiagnosticsWith.apply(book);
    }

    @Override
    int bookClose(Pointer book) {
      int status = bookCloseWith == null ? super.bookClose(book) : bookCloseWith.applyAsInt(book);
      bookCloses.add(book);
      return status;
    }

    @Override
    String reason() {
      return reasonWith == null ? super.reason() : reasonWith.get();
    }
  }
}
