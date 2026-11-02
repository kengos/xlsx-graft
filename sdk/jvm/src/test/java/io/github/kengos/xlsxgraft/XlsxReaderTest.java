package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.Duration;
import java.time.LocalDate;
import java.time.LocalDateTime;
import java.time.LocalTime;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.NoSuchElementException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.function.Function;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class XlsxReaderTest {

  private static final Map<String, String> NO_ENV = Collections.emptyMap();

  private static final String DATA =
      "<row r=\"1\"><c r=\"A1\" t=\"s\"><v>1</v></c><c r=\"C1\"><v>3</v></c>"
          + "<c r=\"D1\" t=\"b\"><v>1</v></c></row>"
          + "<row r=\"3\"><c r=\"B3\" s=\"1\"><v>45322</v></c><c r=\"C3\" s=\"2\"><v>0.75</v></c>"
          + "<c r=\"D3\" s=\"3\"><v>45322.000005787037</v></c><c r=\"E3\" s=\"4\"><v>1.5</v></c>"
          + "<c r=\"F3\" t=\"e\"><v>#N/A</v></c><c r=\"G3\" t=\"inlineStr\"><is><t></t></is></c></row>"
          + "<row r=\"4\"/>";

  static final byte[] BYTES =
      Workbook.xlsx(
          Workbook.worksheet("Data", DATA),
          Workbook.chartsheet("Chart"),
          Workbook.worksheet("Hidden", "", "veryHidden"));

  @Test
  void openBytesGivesEachRowWithItsNumberAndItsValuesFromColumnA() {
    Read read = read(BYTES, book -> book.sheet("data"));

    assertTrue(read.result.success());
    assertFalse(read.result.failed());
    assertNull(read.result.value());
    assertNull(read.result.error());
    assertTrue(read.result.diagnostics().isEmpty());
    assertEquals(Arrays.asList(1, 3, 4), numbers(read.rows));
    assertEquals(Arrays.asList("b", null, 3.0, true), read.rows.get(0).values());
    assertEquals(
        Arrays.asList(
            null,
            LocalDate.of(2024, 1, 31),
            LocalTime.of(18, 0),
            LocalDateTime.of(2024, 1, 31, 0, 0, 0, 500_000_000),
            Duration.ofHours(36),
            new CellError("#N/A"),
            ""),
        read.rows.get(1).values());
    assertTrue(read.rows.get(2).values().isEmpty());
    assertThrows(UnsupportedOperationException.class, () -> read.rows.get(0).values().clear());
  }

  @Test
  void sheetsListsTheSheets() {
    List<SheetInfo> sheets;
    try (Book book = XlsxReader.openBytes(BYTES)) {
      sheets = book.sheets();
    }

    assertEquals(
        Arrays.asList(
            new SheetInfo(0, "Data", SheetKind.WORKSHEET, SheetState.VISIBLE),
            new SheetInfo(1, "Chart", SheetKind.CHARTSHEET, SheetState.VISIBLE),
            new SheetInfo(2, "Hidden", SheetKind.WORKSHEET, SheetState.VERY_HIDDEN)),
        sheets);
    SheetInfo hidden = sheets.get(2);
    assertEquals(
        Arrays.asList(2, "Hidden", SheetKind.WORKSHEET, SheetState.VERY_HIDDEN),
        Arrays.asList(hidden.index(), hidden.name(), hidden.kind(), hidden.state()));
    assertEquals("2 Hidden (WORKSHEET, VERY_HIDDEN)", hidden.toString());
    assertEquals(
        new SheetInfo(2, "Hidden", SheetKind.WORKSHEET, SheetState.VERY_HIDDEN).hashCode(),
        hidden.hashCode());
    assertNotEquals(new SheetInfo(2, "Hidden", SheetKind.WORKSHEET, SheetState.HIDDEN), hidden);
    assertNotEquals(
        new SheetInfo(2, "Hidden", SheetKind.CHARTSHEET, SheetState.VERY_HIDDEN), hidden);
    assertNotEquals(
        new SheetInfo(2, "hidden", SheetKind.WORKSHEET, SheetState.VERY_HIDDEN), hidden);
    assertNotEquals(
        new SheetInfo(1, "Hidden", SheetKind.WORKSHEET, SheetState.VERY_HIDDEN), hidden);
    assertFalse(hidden.equals("Hidden"));
  }

  @Test
  void eachCallOfRowsReadsTheSheetAgainFromItsStartAndIteratesOnce() {
    try (Book book = XlsxReader.openBytes(BYTES)) {
      Rows rows = book.sheet().rows();
      Iterator<Row> first = rows.iterator();

      assertEquals(1, first.next().number());
      assertEquals(Arrays.asList(1, 3, 4), numbers(book.sheet(0).rows()));
      assertThrows(IllegalStateException.class, rows::iterator);
    }
  }

  @Test
  void anIteratorPastItsLastRowThrowsNoSuchElement() {
    try (Book book = XlsxReader.openBytes(BYTES)) {
      Iterator<Row> rows = book.sheet("Hidden").rows().iterator();

      assertFalse(rows.hasNext());
      assertThrows(NoSuchElementException.class, rows::next);
    }
  }

  @Test
  void openBytesCopiesTheBytesWhenCalled() {
    byte[] data = BYTES.clone();
    try (Book book = XlsxReader.openBytes(data)) {
      Arrays.fill(data, (byte) 0);

      assertEquals(3, book.sheets().size());
    }
  }

  @ParameterizedTest
  @CsvSource(
      delimiter = '|',
      value = {
        "Missing | sheet_not_found | no sheet is named \"Missing\"",
        "9 | sheet_not_found | there is no sheet at index 9: the workbook has 3, from index 0",
        "Chart | sheet_not_worksheet | sheet \"Chart\" is a chartsheet, which holds no cells",
        "1 | sheet_not_worksheet | sheet \"Chart\" is a chartsheet, which holds no cells",
      })
  void aRefusedSheetFailsReadingNoRows(String which, String code, String message) {
    Read read =
        read(
            BYTES,
            book ->
                Character.isDigit(which.charAt(0))
                    ? book.sheet(Integer.parseInt(which))
                    : book.sheet(which));

    assertFalse(read.result.success());
    assertTrue(read.rows.isEmpty());
    assertEquals(StatusDiagnostic.error(code, message), read.result.error());
  }

  @Test
  void aRefusedSheetGoesOnWithTheBlockAndStaysTheError() {
    Book book = XlsxReader.openBytes(BYTES);
    try (Book open = book) {
      assertTrue(numbers(open.sheet("Missing").rows()).isEmpty());
      assertEquals(3, numbers(open.sheet("Data").rows()).size());
    }

    assertEquals("sheet_not_found", book.result().error().code());
  }

  @Test
  void aSheetWhosePartIsNotKnownFailsWithSheetUnreadable() {
    Read read = read(Workbook.xlsx(Workbook.unrelated("Lost")), book -> book.sheet("Lost"));

    assertEquals(
        StatusDiagnostic.error(
            "sheet_unreadable", "sheet \"Lost\" cannot be read; the diagnostics say why"),
        read.result.error());
    assertFalse(read.result.diagnostics().isEmpty());
  }

  @Test
  void twoSheetsOfTheNameFailWithSheetAmbiguous() {
    byte[] twice = Workbook.xlsx(Workbook.worksheet("Data", ""), Workbook.worksheet("DATA", ""));
    Read read = read(twice, book -> book.sheet("data"));

    assertEquals(
        "more than one sheet is named \"data\", letter case aside", read.result.error().message());
    assertTrue(codes(read.result).contains("sheet_name_duplicate"));
  }

  @Test
  void aSheetNameThatIsNotUnicodeFailsWithInvalidArg() {
    Read read = read(BYTES, book -> book.sheet("\udcff"));

    assertTrue(read.rows.isEmpty());
    assertEquals("invalid_arg", read.result.error().code());
  }

  @Test
  void bytesThatAreNoWorkbookFailWithWorkbookUnreadableListingNoSheets() {
    Book book = XlsxReader.openBytes("not a zip".getBytes(java.nio.charset.StandardCharsets.UTF_8));
    try (Book open = book) {
      assertTrue(open.sheets().isEmpty());
      assertTrue(numbers(open.sheet().rows()).isEmpty());
    }

    Diagnostic error = book.result().error();
    assertEquals("workbook_unreadable", error.code());
    assertTrue(error.message().startsWith("the workbook cannot be read"), error.message());
    assertFalse(book.result().diagnostics().isEmpty());
  }

  @Test
  void aProblemThatStopsTheReadingFailsWithoutAnError() {
    String twice = "<row r=\"1\"><c r=\"A1\"><v>1</v></c></row><row r=\"1\"/>";
    Read read = read(Workbook.xlsx(Workbook.worksheet("Data", twice)), Book::sheet);

    assertEquals(Collections.singletonList(1), numbers(read.rows));
    assertFalse(read.result.success());
    assertNull(read.result.error());
    Diagnostic first = read.result.diagnostics().get(0);
    assertEquals(
        Arrays.asList("cell_order_invalid", "error"),
        Arrays.asList(first.code(), first.severity()));
    assertTrue(String.valueOf(first.location().get("part")).endsWith("sheet1.xml"));
  }

  @Test
  void anExceptionTheBlockThrowsPropagatesTheBookClosedFirst() {
    Book[] book = new Book[1];

    IllegalArgumentException error =
        assertThrows(
            IllegalArgumentException.class,
            () -> {
              try (Book open = XlsxReader.openBytes(BYTES)) {
                book[0] = open;
                throw new IllegalArgumentException("boom");
              }
            });

    assertEquals("boom", error.getMessage());
    assertTrue(book[0].result().success());
    assertThrows(IllegalStateException.class, book[0]::sheets);
    assertThrows(IllegalStateException.class, () -> book[0].sheet("Data"));
    assertThrows(IllegalStateException.class, () -> book[0].sheet(0));
  }

  @Test
  void aRowAskedForAfterTheBookIsClosedThrows() {
    Rows rows;
    Iterator<Row> started;
    try (Book book = XlsxReader.openBytes(BYTES)) {
      rows = book.sheet().rows();
      started = book.sheet().rows().iterator();
      assertTrue(started.hasNext());
      started.next();
    }

    assertThrows(IllegalStateException.class, () -> rows.iterator().hasNext());
    assertThrows(IllegalStateException.class, started::hasNext);
    rows.close();
  }

  @Test
  void resultThrowsBeforeTheBookIsClosed() {
    try (Book book = XlsxReader.openBytes(BYTES)) {
      IllegalStateException error = assertThrows(IllegalStateException.class, book::result);
      assertTrue(error.getMessage().contains("once the book is closed"), error.getMessage());
    }
  }

  @Test
  void sheetThrowsForANullNameOrAnIndexBelow0() {
    try (Book book = XlsxReader.openBytes(BYTES)) {
      assertThrows(NullPointerException.class, () -> book.sheet(null));
      assertThrows(IllegalArgumentException.class, () -> book.sheet(-1));
    }
  }

  @Test
  void aSecondCloseDoesNothing() {
    Book book = XlsxReader.openBytes(BYTES);
    book.close();
    book.close();

    assertTrue(book.result().success());
  }

  @Test
  void openReadsTheWorkbookAtAPath() throws IOException {
    Path path = Files.createTempFile("book", ".xlsx");
    Files.write(path, BYTES);
    try {
      Book book = XlsxReader.open(path);
      try (Book open = book) {
        assertEquals(3, numbers(open.sheet().rows()).size());
      }

      assertTrue(book.result().success());
    } finally {
      Files.delete(path);
    }
  }

  @Test
  void aFileThatCannotBeOpenedFailsWithIoSayingWhy() {
    Book book = XlsxReader.open(Paths.get("/nowhere/book.xlsx"));
    try (Book open = book) {
      assertTrue(open.sheets().isEmpty());
    }

    assertFalse(book.result().success());
    assertTrue(book.result().diagnostics().isEmpty());
    assertEquals("io", book.result().error().code());
    assertTrue(
        book.result().error().message().contains("nowhere"), book.result().error().message());
  }

  @Test
  void aPathThatIsNotUnicodeFailsWithInvalidArg() {
    Book book = XlsxReader.open("/tmp/\udcff.xlsx", null, System.getenv());
    book.close();

    assertEquals("invalid_arg", book.result().error().code());
  }

  @Test
  void openTakesTheLibraryArgumentWhenXlsxGraftLibraryIsUnset() {
    XlsxReader.open(Paths.get("/nowhere.xlsx"), Repo.ENGINE_LIBRARY, NO_ENV).close();
    XlsxReader.openBytes(BYTES, Repo.ENGINE_LIBRARY, NO_ENV).close();
    XlsxReader.open(Paths.get("/nowhere.xlsx"), "/elsewhere.so").close();
    XlsxReader.openBytes(BYTES, "/elsewhere.so").close();

    assertThrows(
        LibraryNotFoundException.class,
        () -> XlsxReader.open(Paths.get("/nowhere.xlsx"), "/elsewhere.so", NO_ENV));
    assertThrows(
        LibraryNotFoundException.class, () -> XlsxReader.openBytes(BYTES, "/elsewhere.so", NO_ENV));
    assertThrows(NullPointerException.class, () -> XlsxReader.open(null));
  }

  @Test
  void twoIteratorsReadInTurns() {
    try (Book book = XlsxReader.openBytes(BYTES)) {
      Iterator<Row> one = book.sheet().rows().iterator();
      Iterator<Row> other = book.sheet("Data").rows().iterator();
      List<Row> first = new ArrayList<>();
      while (one.hasNext()) {
        Row row = one.next();
        Row same = other.next();
        assertEquals(row, same);
        assertEquals(row.hashCode(), same.hashCode());
        assertEquals(row.toString(), same.toString());
        first.add(row);
      }

      assertFalse(other.hasNext());
      assertEquals("1: [b, null, 3.0, true]", first.get(0).toString());
      assertNotEquals(first.get(0), first.get(1));
      assertNotEquals(first.get(2), new Row(5, first.get(2).values()));
      assertFalse(first.get(0).equals("1"));
    }
  }

  @Test
  void threadsReadOneBookAtOnce() throws Exception {
    Book book = XlsxReader.openBytes(BYTES);
    ExecutorService pool = Executors.newFixedThreadPool(8);
    try (Book open = book) {
      List<Future<Integer>> counts = new ArrayList<>();
      for (int i = 0; i < 80; i++) {
        counts.add(pool.submit(() -> numbers(open.sheet().rows()).size()));
      }
      for (Future<Integer> count : counts) {
        assertEquals(3, count.get());
      }
    } finally {
      pool.shutdown();
    }

    assertTrue(book.result().success());
  }

  @Test
  void cellErrorsAreEqualByTheirText() {
    CellError na = new CellError("#N/A");

    assertEquals("#N/A", na.text());
    assertEquals("#N/A", na.toString());
    assertEquals(new CellError("#N/A").hashCode(), na.hashCode());
    assertNotEquals(new CellError("#DIV/0!"), na);
    assertFalse(na.equals("#N/A"));
  }

  static List<Integer> numbers(Iterable<Row> rows) {
    List<Integer> numbers = new ArrayList<>();
    for (Row row : rows) {
      numbers.add(row.number());
    }
    return numbers;
  }

  private static List<String> codes(Result<Void> result) {
    List<String> codes = new ArrayList<>();
    for (Diagnostic diagnostic : result.diagnostics()) {
      codes.add(diagnostic.code());
    }
    return codes;
  }

  private static Read read(byte[] data, Function<Book, Sheet> which) {
    Book book = XlsxReader.openBytes(data);
    List<Row> rows = new ArrayList<>();
    try (Book open = book;
        Rows read = which.apply(open).rows()) {
      for (Row row : read) {
        rows.add(row);
      }
    }
    return new Read(book.result(), rows);
  }

  /** A sheet's rows and the result after its book closed. */
  private static final class Read {

    final Result<Void> result;
    final List<Row> rows;

    Read(Result<Void> result, List<Row> rows) {
      this.result = result;
      this.rows = rows;
    }
  }
}
