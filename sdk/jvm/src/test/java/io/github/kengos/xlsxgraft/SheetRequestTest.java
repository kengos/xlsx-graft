package io.github.kengos.xlsxgraft;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Arrays;
import java.util.Collections;
import org.junit.jupiter.api.Test;

class SheetRequestTest {

  @Test
  void firstGivesNoKeySoTheEngineShowsTheFirstSheet() {
    assertTrue(SheetRequest.first().json().isEmpty());
  }

  @Test
  void eachValueGivenIsAKeyOfTheEnginesNameInTheOrderSheetSheetIndexRangeMaxRowsMaxColumns() {
    SheetRequest request =
        SheetRequest.first().maxColumns(5).maxRows(4).range("A1:C3").sheetIndex(2).sheet("Data");

    assertEquals(
        Arrays.asList("sheet", "sheetIndex", "range", "maxRows", "maxColumns"),
        Arrays.asList(request.json().keySet().toArray()));
    assertEquals("Data", request.json().get("sheet"));
    assertEquals(2, request.json().get("sheetIndex"));
    assertEquals("A1:C3", request.json().get("range"));
    assertEquals(4, request.json().get("maxRows"));
    assertEquals(5, request.json().get("maxColumns"));
  }

  @Test
  void aNullGivenAfterAValueTakesTheKeyAwayAndNothingIsChecked() {
    SheetRequest request =
        SheetRequest.first()
            .sheet("Data")
            .sheetIndex(1)
            .range("A1")
            .maxRows(0)
            .maxColumns(-1)
            .sheet(null)
            .range(null)
            .maxRows(null);

    assertEquals(
        Arrays.asList("sheetIndex", "maxColumns"),
        Arrays.asList(request.json().keySet().toArray()));
    assertEquals(-1, request.json().get("maxColumns"));
  }

  @Test
  void aRequestDoesNotChangeWhenAnotherIsMadeFromIt() {
    SheetRequest first = SheetRequest.first();
    SheetRequest named = first.sheet("Data");
    SheetRequest windowed = named.maxRows(2);

    assertEquals(Collections.emptyMap(), first.json());
    assertEquals(Arrays.asList("sheet"), Arrays.asList(named.json().keySet().toArray()));
    assertEquals(
        Arrays.asList("sheet", "maxRows"), Arrays.asList(windowed.json().keySet().toArray()));
  }
}
