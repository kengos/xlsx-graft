package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class ViewContentTest {

  // Every string and number is different from every other, so that reading a key into another
  // field changes some value. The flags of the parts a, b and c take true, false and (for
  // referenced) null so that every flag is true somewhere and false somewhere, and no two flags are
  // equal in every part.
  private static final String REL =
      "{\"id\":\"rId1\",\"type\":\"type-1\",\"target\":{\"part\":\"/target-1\"}}";
  private static final String EXTERNAL =
      "{\"id\":\"rId2\",\"type\":\"type-2\",\"target\":{\"external\":\"external-2\"}}";
  private static final String PART =
      "{\"name\":\"/part-a\",\"contentType\":\"type/a\","
          + "\"contentTypeKnown\":true,\"size\":120,\"compressedSize\":80,\"readable\":false,"
          + "\"referenced\":true,\"relationshipsKnown\":false,\"relationships\":["
          + EXTERNAL
          + "]}";
  private static final String UNKNOWN_PART =
      "{\"name\":\"/part-b\",\"contentType\":null,\"contentTypeKnown\":false,\"size\":13,"
          + "\"compressedSize\":5,\"readable\":true,\"referenced\":null,"
          + "\"relationshipsKnown\":false,\"relationships\":["
          + "{\"id\":\"rId3\",\"type\":\"type-3\",\"target\":{\"part\":\"/target-3\"}}]}";

  private static final String PART_C =
      "{\"name\":\"/part-c\",\"contentType\":\"type/c\",\"contentTypeKnown\":false,\"size\":17,"
          + "\"compressedSize\":9,\"readable\":false,\"referenced\":false,"
          + "\"relationshipsKnown\":true,\"relationships\":[]}";

  @Test
  void everyKeyOfATreeIsReadIntoItsOwnField() {
    Tree tree = tree("true", "false", "[" + REL + "]", "[" + PART + "," + UNKNOWN_PART + "]");

    assertTrue(tree.complete());
    assertFalse(tree.relationshipsKnown());
    assertEquals(1, tree.relationships().size());
    assertRelationship(tree.relationships().get(0), "rId1", "type-1", "/target-1", null);
    assertEquals(2, tree.parts().size());
    TreePart a = tree.parts().get(0);
    assertEquals("/part-a", a.name());
    assertEquals("type/a", a.contentType());
    assertTrue(a.contentTypeKnown());
    assertEquals(120L, a.size());
    assertEquals(80L, a.compressedSize());
    assertFalse(a.readable());
    assertEquals(Boolean.TRUE, a.referenced());
    assertFalse(a.relationshipsKnown());
    assertEquals(1, a.relationships().size());
    assertRelationship(a.relationships().get(0), "rId2", "type-2", null, "external-2");
    TreePart b = tree.parts().get(1);
    assertEquals("/part-b", b.name());
    assertNull(b.contentType());
    assertFalse(b.contentTypeKnown());
    assertEquals(13L, b.size());
    assertEquals(5L, b.compressedSize());
    assertTrue(b.readable());
    assertNull(b.referenced());
    assertFalse(b.relationshipsKnown());
    assertEquals(1, b.relationships().size());
    assertRelationship(b.relationships().get(0), "rId3", "type-3", "/target-3", null);
  }

  @Test
  void aTreeWhoseFlagsAreTheOtherWayIsReadIntoItsOwnFields() {
    Tree tree = tree("false", "true", "[" + REL + "]", "[" + PART_C + "]");

    assertFalse(tree.complete());
    assertTrue(tree.relationshipsKnown());
    assertEquals(1, tree.relationships().size());
    assertRelationship(tree.relationships().get(0), "rId1", "type-1", "/target-1", null);
    assertEquals(1, tree.parts().size());
    TreePart c = tree.parts().get(0);
    assertEquals("/part-c", c.name());
    assertEquals("type/c", c.contentType());
    assertFalse(c.contentTypeKnown());
    assertEquals(17L, c.size());
    assertEquals(9L, c.compressedSize());
    assertFalse(c.readable());
    assertEquals(Boolean.FALSE, c.referenced());
    assertTrue(c.relationshipsKnown());
    assertTrue(c.relationships().isEmpty());
  }

  @Test
  void aTreesListsCannotBeChanged() {
    Tree tree = tree("true", "true", "[" + REL + "]", "[" + PART + "]");

    assertThrows(UnsupportedOperationException.class, () -> tree.parts().clear());
    assertThrows(UnsupportedOperationException.class, () -> tree.relationships().clear());
    assertThrows(
        UnsupportedOperationException.class, () -> tree.parts().get(0).relationships().clear());
  }

  @Test
  void everyKeyOfAGridIsReadIntoItsOwnField() {
    Grid grid =
        grid(
            "{\"index\":4,\"name\":\"Data\",\"kind\":\"dialogsheet\",\"state\":\"hidden\"}",
            "[{\"row\":3,\"cells\":["
                + "{\"ref\":\"A3\",\"type\":\"string\",\"value\":\"text-a3\",\"truncated\":true},"
                + "{\"ref\":\"B3\",\"type\":\"boolean\",\"value\":true}]},"
                + "{\"row\":7,\"cells\":["
                + "{\"ref\":\"C7\",\"type\":\"elapsed\",\"value\":1500},"
                + "{\"ref\":\"D7\",\"type\":\"number\",\"value\":1.5},"
                + "{\"ref\":\"E7\",\"type\":\"number\",\"value\":2.0},"
                + "{\"ref\":\"F7\",\"type\":\"date\",\"value\":\"2024-01-15\"}]}]");

    assertEquals(4, grid.sheet().index());
    assertEquals("Data", grid.sheet().name());
    assertEquals(SheetKind.DIALOGSHEET, grid.sheet().kind());
    assertEquals(SheetState.HIDDEN, grid.sheet().state());
    assertEquals("A1:E9", grid.dimension());
    assertEquals("A1:D8", grid.used());
    assertEquals("A1:C3", grid.shown());
    assertTrue(grid.truncated().rows());
    assertFalse(grid.truncated().columns());
    assertFalse(grid.truncated().merges());
    assertFalse(grid.complete());
    assertEquals(Arrays.asList("A1:B1", "C2:D2"), grid.merges());
    assertEquals(2, grid.rows().size());
    GridRow first = grid.rows().get(0);
    GridRow second = grid.rows().get(1);
    assertEquals(3, first.row());
    assertEquals(7, second.row());
    assertCell(first.cells().get(0), "A3", "string", "text-a3", true);
    assertCell(first.cells().get(1), "B3", "boolean", Boolean.TRUE, false);
    assertEquals(4, second.cells().size());
    assertCell(second.cells().get(0), "C7", "elapsed", 1500L, false);
    assertCell(second.cells().get(1), "D7", "number", 1.5, false);
    assertCell(second.cells().get(2), "E7", "number", 2.0, false);
    assertCell(second.cells().get(3), "F7", "date", "2024-01-15", false);
    assertThrows(UnsupportedOperationException.class, () -> grid.rows().clear());
    assertThrows(UnsupportedOperationException.class, () -> grid.merges().clear());
    assertThrows(UnsupportedOperationException.class, () -> first.cells().clear());
  }

  @Test
  void aGridWhoseRangesAreNullHasNulls() {
    String body =
        gridBody("null", "[]")
            .replace("\"dimension\":\"A1:E9\"", "\"dimension\":null")
            .replace("\"used\":\"A1:D8\"", "\"used\":null")
            .replace("\"shown\":\"A1:C3\"", "\"shown\":null");

    Grid grid = read(withEnvelope(body), ViewContent::grid).value();

    assertNull(grid.dimension());
    assertNull(grid.used());
    assertNull(grid.shown());
  }

  @Test
  void eachTruncatedFlagIsReadFromItsOwnKeyAndCompleteFromItsOwn() {
    GridTruncated onlyRows = truncated("{\"rows\":true,\"columns\":false,\"merges\":false}");
    Grid complete = completeGrid("{\"rows\":false,\"columns\":true,\"merges\":false}");
    GridTruncated onlyMerges = truncated("{\"rows\":false,\"columns\":false,\"merges\":true}");

    assertTrue(onlyRows.rows());
    assertFalse(onlyRows.columns() || onlyRows.merges());
    assertTrue(complete.truncated().columns());
    assertFalse(complete.truncated().rows() || complete.truncated().merges());
    assertTrue(complete.complete());
    assertTrue(onlyMerges.merges());
    assertFalse(onlyMerges.rows() || onlyMerges.columns());
  }

  @Test
  void aGridWithNoSheetHasNullForItAndTheKindsAndStatesAreNamedByTheirStrings() {
    Grid none = grid("null", "[]");
    String[] kinds = {"worksheet", "chartsheet", "dialogsheet", "macrosheet", "newer"};
    SheetKind[] kindsRead = {
      SheetKind.WORKSHEET,
      SheetKind.CHARTSHEET,
      SheetKind.DIALOGSHEET,
      SheetKind.MACROSHEET,
      SheetKind.UNKNOWN
    };
    String[] states = {"visible", "hidden", "veryHidden", "newer"};
    SheetState[] statesRead = {
      SheetState.VISIBLE, SheetState.HIDDEN, SheetState.VERY_HIDDEN, SheetState.UNKNOWN
    };

    assertNull(none.sheet());
    for (int i = 0; i < kinds.length; i++) {
      assertEquals(kindsRead[i], SheetKind.named(kinds[i]));
    }
    for (int i = 0; i < states.length; i++) {
      assertEquals(statesRead[i], SheetState.named(states[i]));
    }
  }

  @ParameterizedTest
  @ValueSource(
      strings = {
        "{\"complete\":true}",
        "{\"complete\":1,\"relationshipsKnown\":true,\"relationships\":[],\"parts\":[]}",
        "{\"complete\":true,\"relationshipsKnown\":true,\"relationships\":{},\"parts\":[]}",
        "{\"complete\":true,\"relationshipsKnown\":true,\"relationships\":[],\"parts\":[3]}",
        "{\"complete\":true,\"relationshipsKnown\":true,\"relationships\":[{\"id\":\"a\"}],\"parts\":[]}",
        "{\"complete\":true,\"relationshipsKnown\":true,\"relationships\":[{\"id\":\"a\",\"type\":\"t\",\"target\":3}],\"parts\":[]}",
        "{\"complete\":true,\"relationshipsKnown\":true,\"relationships\":[{\"id\":\"a\",\"type\":\"t\",\"target\":{\"part\":3}}],\"parts\":[]}",
        "{\"complete\":true,\"relationshipsKnown\":true,\"relationships\":[],\"parts\":[{\"name\":\"/x\"}]}",
      })
  void aTreeOfAnotherShapeIsUnreadable(String content) {
    Result<Tree> result = read(withEnvelope(content), ViewContent::tree);

    assertUnreadable(result);
  }

  @Test
  void aPartWhoseSizeIsNotAWholeNumberOrAFlagIsNotABooleanIsUnreadable() {
    assertUnreadable(treeResult(PART.replace("\"size\":120", "\"size\":\"120\"")));
    assertUnreadable(treeResult(PART.replace("\"size\":120", "\"size\":1.5")));
    assertUnreadable(treeResult(PART.replace("\"readable\":false", "\"readable\":null")));
    assertUnreadable(treeResult(PART.replace("\"referenced\":true", "\"referenced\":1")));
    assertUnreadable(treeResult(PART.replace("\"contentType\":\"type/a\"", "\"contentType\":3")));
  }

  @Test
  void aGridOfAnotherShapeIsUnreadable() {
    String sheet = "{\"index\":0,\"name\":\"a\",\"kind\":\"worksheet\",\"state\":\"visible\"}";

    assertUnreadable(gridResult("3", "[]"));
    assertUnreadable(gridResult("{\"index\":0,\"name\":\"a\",\"kind\":\"worksheet\"}", "[]"));
    assertUnreadable(gridResult(sheet.replace("\"index\":0", "\"index\":3000000000"), "[]"));
    assertUnreadable(gridResult(sheet.replace("\"index\":0", "\"index\":\"0\""), "[]"));
    assertUnreadable(gridResult(sheet.replace("\"kind\":\"worksheet\"", "\"kind\":1"), "[]"));
    assertUnreadable(gridResult(sheet, "[{\"row\":3000000000,\"cells\":[]}]"));
    assertUnreadable(gridResult(sheet, "[{\"row\":1}]"));
    assertUnreadable(
        gridResult(sheet, "[{\"row\":1,\"cells\":[{\"ref\":\"A1\",\"type\":\"string\"}]}]"));
    assertUnreadable(
        gridResult(
            sheet,
            "[{\"row\":1,\"cells\":[{\"ref\":\"A1\",\"type\":\"string\",\"value\":\"a\",\"truncated\":1}]}]"));
    assertUnreadable(
        gridResult(sheet, "[{\"row\":1,\"cells\":[{\"type\":\"string\",\"value\":\"a\"}]}]"));
    assertUnreadable(read(withEnvelope("{}"), ViewContent::grid));
  }

  @Test
  void aGridWhoseMergesHoldANumberOrWhoseTruncatedLacksAKeyIsUnreadable() {
    String body = gridBody("null", "[]");

    assertUnreadable(
        read(withEnvelope(body.replace("[\"A1:B1\",\"C2:D2\"]", "[1]")), ViewContent::grid));
    assertUnreadable(read(withEnvelope(body.replace("\"columns\":false,", "")), ViewContent::grid));
  }

  private static void assertUnreadable(Result<?> result) {
    assertFalse(result.success());
    assertNull(result.value());
    assertEquals("diagnostics_unreadable", result.error().code());
  }

  private static Tree tree(String complete, String known, String relationships, String parts) {
    return treeResult(complete, known, relationships, parts).value();
  }

  private static Result<Tree> treeResult(String part) {
    return treeResult("true", "true", "[]", "[" + part + "]");
  }

  private static Result<Tree> treeResult(
      String complete, String known, String relationships, String parts) {
    return read(
        withEnvelope(
            "{\"complete\":"
                + complete
                + ",\"relationshipsKnown\":"
                + known
                + ",\"relationships\":"
                + relationships
                + ",\"parts\":"
                + parts
                + "}"),
        ViewContent::tree);
  }

  private static String gridBody(String sheet, String rows) {
    return "{\"sheet\":"
        + sheet
        + ",\"dimension\":\"A1:E9\",\"used\":\"A1:D8\",\"shown\":\"A1:C3\","
        + "\"truncated\":{\"rows\":true,\"columns\":false,\"merges\":false},"
        + "\"complete\":false,\"rows\":"
        + rows
        + ",\"merges\":[\"A1:B1\",\"C2:D2\"]}";
  }

  private static void assertRelationship(
      TreeRelationship relationship, String id, String type, String part, String external) {
    assertEquals(id, relationship.id());
    assertEquals(type, relationship.type());
    assertEquals(part, relationship.target().part());
    assertEquals(external, relationship.target().external());
  }

  private static void assertCell(
      GridCell cell, String ref, String type, Object value, boolean truncated) {
    assertEquals(ref, cell.ref());
    assertEquals(type, cell.type());
    assertEquals(value, cell.value());
    assertEquals(truncated, cell.truncated());
  }

  // A grid whose `complete` is true, with `flags` as its truncated.
  private static Grid completeGrid(String flags) {
    String body =
        gridBody("null", "[]")
            .replace("{\"rows\":true,\"columns\":false,\"merges\":false}", flags)
            .replace("\"complete\":false", "\"complete\":true");
    return read(withEnvelope(body), ViewContent::grid).value();
  }

  private static GridTruncated truncated(String flags) {
    String body =
        gridBody("null", "[]").replace("{\"rows\":true,\"columns\":false,\"merges\":false}", flags);
    return read(withEnvelope(body), ViewContent::grid).value().truncated();
  }

  private static Grid grid(String sheet, String rows) {
    return gridResult(sheet, rows).value();
  }

  private static Result<Grid> gridResult(String sheet, String rows) {
    return read(withEnvelope(gridBody(sheet, rows)), ViewContent::grid);
  }

  // The view's JSON: the keys of `ok`, `error` and `diagnostics`, then those of `content`.
  private static String withEnvelope(String content) {
    String keys = content.substring(1);
    return "{\"ok\":true,\"error\":null,\"diagnostics\":[]" + (keys.equals("}") ? "" : ",") + keys;
  }

  private static <T> Result<T> read(String json, ViewReport.Content<T> content) {
    return ViewReport.call(
        new ViewCall.Outcome(0, json.getBytes(StandardCharsets.UTF_8), null), content);
  }
}
