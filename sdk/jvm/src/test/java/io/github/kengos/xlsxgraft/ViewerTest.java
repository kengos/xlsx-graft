package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import org.junit.jupiter.api.Test;

class ViewerTest {

  private static final Map<String, String> NO_ENV = Collections.emptyMap();
  private static final Path TABLE = Repo.ROOT.resolve("fixtures/table.xlsx");
  private static final Path MERGED = Repo.ROOT.resolve("fixtures/merged.xlsx");
  private static final Path EXTERNAL = Repo.ROOT.resolve("fixtures/external-link.xlsx");

  private final Viewer viewer = new Viewer(Repo.ENGINE_LIBRARY, NO_ENV);

  @Test
  void treeListsThePartsAndTheRelationshipsOfTheWorkbookAtAPath() {
    Result<Tree> result = viewer.tree(TABLE);

    assertTrue(result.success());
    assertNull(result.error());
    Tree tree = result.value();
    assertTrue(tree.complete());
    assertTrue(partNames(tree).contains("/xl/workbook.xml"));
    List<String> targets = new ArrayList<>();
    for (TreeRelationship relationship : tree.relationships()) {
      targets.add(relationship.target().part());
    }
    assertTrue(targets.contains("/xl/workbook.xml"), targets.toString());
  }

  @Test
  void treeGivesTheSameTreeForTheBytesOfTheFile() throws IOException {
    assertEquals(
        partNames(viewer.tree(TABLE).value()),
        partNames(viewer.treeBytes(Files.readAllBytes(TABLE)).value()));
  }

  @Test
  void treeGivesATargetOutsideThePackageAsExternalWithNoPart() {
    Tree tree = viewer.tree(EXTERNAL).value();

    TreeTarget external = null;
    for (TreePart part : tree.parts()) {
      for (TreeRelationship relationship : part.relationships()) {
        if (relationship.target().external() != null) {
          external = relationship.target();
        }
      }
    }

    assertNotNull(external);
    assertNull(external.part());
    assertFalse(external.external().isEmpty());
  }

  @Test
  void treeFailsWithAnIoErrorAndATreeWithNoPartsForAFileThatDoesNotExist() {
    Result<Tree> result = viewer.tree(Paths.get("/nowhere/book.xlsx"));

    assertFalse(result.success());
    assertEquals("io", result.error().code());
    assertTrue(result.value().parts().isEmpty());
  }

  @Test
  void treeBytesFailsWithDiagnosticsAndATreeWithNoPartsForBytesThatAreNoWorkbook() {
    Result<Tree> result = viewer.treeBytes("not a zip".getBytes(StandardCharsets.UTF_8));

    assertFalse(result.success());
    assertNull(result.error());
    assertFalse(result.diagnostics().isEmpty());
    assertTrue(result.value().parts().isEmpty());
  }

  @Test
  void aFileWhosePathIsNotUnicodeIsReadAndItsBytesShown() {
    Result<Tree> tree = viewer.tree(NonUnicodePath.standingFor(TABLE));
    Result<Grid> grid = viewer.describeSheet(NonUnicodePath.standingFor(MERGED));
    Result<Tree> missing = viewer.tree(NonUnicodePath.standingFor(Paths.get("/nowhere/book.xlsx")));
    Result<Grid> missingGrid =
        viewer.describeSheet(NonUnicodePath.standingFor(Paths.get("/nowhere/book.xlsx")));

    assertEquals(partNames(viewer.tree(TABLE).value()), partNames(tree.value()));
    assertEquals(viewer.describeSheet(MERGED).value().shown(), grid.value().shown());
    assertEquals("io", missing.error().code());
    assertTrue(missing.error().message().startsWith("the file could not be read: "));
    assertNull(missing.value());
    assertEquals("io", missingGrid.error().code());
    assertNull(missingGrid.value());
  }

  @Test
  void describeSheetShowsTheSheetTheRangesAndTheMergesOfTheFirstSheet() {
    Result<Grid> result = viewer.describeSheet(MERGED);

    assertTrue(result.success());
    assertNull(result.error());
    Grid grid = result.value();
    assertEquals(new SheetInfo(0, "Sheet1", SheetKind.WORKSHEET, SheetState.VISIBLE), grid.sheet());
    assertEquals("A1:E8", grid.dimension());
    assertEquals("A1:E8", grid.used());
    assertEquals("A1:E8", grid.shown());
    assertTrue(grid.complete());
    assertFalse(grid.truncated().rows() || grid.truncated().columns() || grid.truncated().merges());
    assertEquals(Arrays.asList("A1:C1", "A3:A5"), grid.merges());
  }

  @Test
  void describeSheetShowsTheCellsOfTheFirstSheet() {
    Grid grid = viewer.describeSheet(MERGED).value();

    assertEquals(1, grid.rows().get(0).row());
    assertEquals("A1", grid.rows().get(0).cells().get(0).ref());
    assertEquals("string", grid.rows().get(0).cells().get(0).type());
    assertEquals("Quarterly report", grid.rows().get(0).cells().get(0).value());
    assertFalse(grid.rows().get(0).cells().get(0).truncated());
    assertEquals(1.5, cell(grid, "C3").value());
    assertEquals(2.0, cell(grid, "C4").value());
    assertEquals("date", cell(grid, "C6").type());
    assertEquals("2024-01-15", cell(grid, "C6").value());
  }

  @Test
  void describeSheetGivesTheSameGridForTheBytesOfTheFile() throws IOException {
    Grid grid = viewer.describeSheetBytes(Files.readAllBytes(MERGED)).value();

    assertEquals(viewer.describeSheet(MERGED).value().shown(), grid.shown());
    assertEquals(Arrays.asList("A1:C1", "A3:A5"), grid.merges());
  }

  @Test
  void describeSheetTakesTheWindowAndTheSheetThatTheRequestNames() throws IOException {
    byte[] data = Files.readAllBytes(MERGED);

    assertEquals(
        "A1:C3", viewer.describeSheet(MERGED, SheetRequest.first().range("A1:C3")).value().shown());
    assertEquals(
        "A1:E2", viewer.describeSheet(MERGED, SheetRequest.first().maxRows(2)).value().shown());
    assertEquals(
        "A1:B8", viewer.describeSheet(MERGED, SheetRequest.first().maxColumns(2)).value().shown());
    assertEquals(
        "Sheet1",
        viewer.describeSheet(MERGED, SheetRequest.first().sheet("Sheet1")).value().sheet().name());
    assertEquals(
        0,
        viewer.describeSheet(MERGED, SheetRequest.first().sheetIndex(0)).value().sheet().index());
    assertEquals(
        "A1:E2",
        viewer
            .describeSheetBytes(data, SheetRequest.first().sheet("Sheet1").maxRows(2))
            .value()
            .shown());
  }

  @Test
  void describeSheetFailsWithSheetNotFoundAndAGridWhoseSheetIsNull() {
    Result<Grid> result = viewer.describeSheet(TABLE, SheetRequest.first().sheet("Nope"));

    assertFalse(result.success());
    assertEquals("sheet_not_found", result.error().code());
    assertNull(result.value().sheet());
    assertTrue(result.value().rows().isEmpty());
  }

  @Test
  void describeSheetThrowsIllegalArgumentWithTheEnginesMessageForAValueTheEngineRefuses() {
    assertTrue(refused(SheetRequest.first().range("nope")).contains("range"));
    assertTrue(refused(SheetRequest.first().maxRows(0)).contains("maxRows"));
    assertTrue(refused(SheetRequest.first().sheet("Sheet1").sheetIndex(0)).contains("sheet"));
    assertTrue(refused(SheetRequest.first().sheetIndex(-1)).contains("sheetIndex"));
  }

  @Test
  void describeSheetThrowsIllegalArgumentBeforeTheCallForOptionsThatCannotBeSent() {
    String huge = repeat("A", 2 << 20);
    Path nowhere = Paths.get("/nowhere/book.xlsx");

    IllegalArgumentException tooLong =
        assertThrows(
            IllegalArgumentException.class,
            () -> viewer.describeSheet(nowhere, SheetRequest.first().range(huge)));
    IllegalArgumentException tooLongBytes =
        assertThrows(
            IllegalArgumentException.class,
            () -> viewer.describeSheetBytes(new byte[0], SheetRequest.first().range(huge)));
    IllegalArgumentException surrogate =
        assertThrows(
            IllegalArgumentException.class,
            () -> viewer.describeSheet(nowhere, SheetRequest.first().range("\ud800")));
    assertThrows(
        IllegalArgumentException.class,
        () -> viewer.describeSheetBytes(new byte[0], SheetRequest.first().sheet("\udc00")));

    assertTrue(tooLong.getMessage().endsWith("the engine takes 2097152"), tooLong.getMessage());
    assertTrue(tooLongBytes.getMessage().endsWith("the engine takes 2097152"));
    assertTrue(surrogate.getMessage().contains("unpaired surrogate"), surrogate.getMessage());
  }

  @Test
  void theOptionsAreBuiltBeforeAPathThatIsNotUnicodeIsRead() {
    Path missing = NonUnicodePath.standingFor(Paths.get("/nowhere/book.xlsx"));

    assertThrows(
        IllegalArgumentException.class,
        () -> viewer.describeSheet(missing, SheetRequest.first().range("\ud800")));
  }

  @Test
  void optionsOfExactlyTwoMebibytesCallTheEngineAndOneByteMoreAreRefused() {
    ViewCallTest.Fake fake = new ViewCallTest.Fake();
    fake.makeStatus = 2;
    fake.writesHandle = false;
    Viewer fakeViewer = new Viewer(fake.call());
    int base = "{\"range\":\"\"}".length();

    Result<Grid> exact =
        fakeViewer.describeSheetBytes(
            new byte[0], SheetRequest.first().range(repeat("A", (2 << 20) - base)));
    IllegalArgumentException over =
        assertThrows(
            IllegalArgumentException.class,
            () ->
                fakeViewer.describeSheetBytes(
                    new byte[0], SheetRequest.first().range(repeat("A", (2 << 20) - base + 1))));

    assertEquals("panic", exact.error().code());
    assertEquals(1, fake.made.size());
    assertTrue(over.getMessage().contains("are 2097153 bytes of JSON"), over.getMessage());
  }

  @Test
  void aNullArgumentThrowsNullPointer() {
    byte[] data = new byte[0];

    assertThrows(NullPointerException.class, () -> viewer.tree(null));
    assertThrows(NullPointerException.class, () -> viewer.treeBytes(null));
    assertThrows(NullPointerException.class, () -> viewer.describeSheet(null));
    assertThrows(NullPointerException.class, () -> viewer.describeSheet(TABLE, null));
    assertThrows(NullPointerException.class, () -> viewer.describeSheetBytes(null));
    assertThrows(NullPointerException.class, () -> viewer.describeSheetBytes(data, null));
  }

  @Test
  void theConstructorsWithFewerArgumentsFindTheLibraryTheEnvironmentNames() {
    // The gate sets XLSX_GRAFT_LIBRARY, which wins over the library argument.
    assertTrue(new Viewer().tree(TABLE).success());
    assertTrue(new Viewer(Repo.ENGINE_LIBRARY).tree(TABLE).success());
  }

  @Test
  void aMissingEngineLibraryThrowsLibraryNotFound() {
    assertThrows(LibraryNotFoundException.class, () -> new Viewer("/elsewhere.so", NO_ENV));
  }

  @Test
  void aPanicOfTheEngineIsAFailureWithThePanicErrorAndNoValue() {
    ViewCallTest.Fake fake = new ViewCallTest.Fake();
    fake.makeStatus = 2;
    fake.writesHandle = false;
    Viewer fakeViewer = new Viewer(fake.call());

    Result<?>[] results = {
      fakeViewer.tree(TABLE),
      fakeViewer.treeBytes(new byte[0]),
      fakeViewer.describeSheet(TABLE),
      fakeViewer.describeSheetBytes(new byte[0])
    };

    for (Result<?> result : results) {
      assertEquals("panic", result.error().code());
      assertTrue(result.diagnostics().isEmpty());
      assertNull(result.value());
    }
  }

  @Test
  void aStatusThePackageDoesNotKnowIsAFailureWithUnknownStatus() {
    ViewCallTest.Fake fake = new ViewCallTest.Fake();
    fake.makeStatus = 99;
    fake.writesHandle = false;

    assertEquals(
        "unknown_status", new Viewer(fake.call()).describeSheetBytes(new byte[0]).error().code());
  }

  @Test
  void anInvalidArgThrowsXlsxGraftException() {
    ViewCallTest.Fake fake = new ViewCallTest.Fake();
    fake.makeStatus = 4;
    fake.writesHandle = false;

    assertThrows(XlsxGraftException.class, () -> new Viewer(fake.call()).treeBytes(new byte[0]));
  }

  @Test
  void oneViewerMayBeSharedByThreadsThatCallItAtOnce() throws Exception {
    byte[] data = Files.readAllBytes(MERGED);
    ExecutorService pool = Executors.newFixedThreadPool(4);
    try {
      List<Future<Grid>> futures = new ArrayList<>();
      for (int thread = 0; thread < 4; thread++) {
        futures.add(pool.submit(() -> viewer.describeSheetBytes(data).value()));
      }
      for (Future<Grid> future : futures) {
        assertEquals("A1:E8", future.get().shown());
        assertFalse(future.get().rows().isEmpty());
      }
    } finally {
      pool.shutdown();
    }
  }

  private String refused(SheetRequest request) {
    return assertThrows(IllegalArgumentException.class, () -> viewer.describeSheet(MERGED, request))
        .getMessage();
  }

  private static GridCell cell(Grid grid, String ref) {
    for (GridRow row : grid.rows()) {
      for (GridCell cell : row.cells()) {
        if (cell.ref().equals(ref)) {
          return cell;
        }
      }
    }
    throw new AssertionError("no cell " + ref);
  }

  private static List<String> partNames(Tree tree) {
    List<String> names = new ArrayList<>();
    for (TreePart part : tree.parts()) {
      names.add(part.name());
    }
    return names;
  }

  private static String repeat(String text, int times) {
    StringBuilder out = new StringBuilder();
    for (int count = 0; count < times; count++) {
      out.append(text);
    }
    return out.toString();
  }
}
