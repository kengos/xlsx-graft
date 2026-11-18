package io.github.kengos.xlsxgraft;

import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Which sheet {@link Viewer#describeSheet(java.nio.file.Path, SheetRequest)} shows and how much of
 * it. Each value is optional, and a value that is not given (or given as null) is not passed to the
 * engine. A value of this class does not change: each method returns a new one.
 *
 * <pre>{@code
 * SheetRequest request = SheetRequest.first().sheet("Data").range("A1:H50").maxRows(20);
 * }</pre>
 *
 * <p>The package does not check a value: one the engine does not take (a {@code range} that is no
 * A1 range, {@code maxRows} out of range, a name together with an index, an index below 0) makes
 * the call throw {@link IllegalArgumentException} with the engine's message.
 */
public final class SheetRequest {

  private static final SheetRequest FIRST = new SheetRequest(null, null, null, null, null);

  private final String sheet;
  private final Integer sheetIndex;
  private final String range;
  private final Integer maxRows;
  private final Integer maxColumns;

  private SheetRequest(
      String sheet, Integer sheetIndex, String range, Integer maxRows, Integer maxColumns) {
    this.sheet = sheet;
    this.sheetIndex = sheetIndex;
    this.range = range;
    this.maxRows = maxRows;
    this.maxColumns = maxColumns;
  }

  /**
   * A request that names nothing: the first sheet, from A1, in the engine's default window.
   *
   * @return a request that passes nothing to the engine
   */
  public static SheetRequest first() {
    return FIRST;
  }

  /**
   * This request with the sheet's name; not together with {@link #sheetIndex(Integer)}.
   *
   * @param sheet the name; null to give none
   * @return the new request
   */
  public SheetRequest sheet(String sheet) {
    return new SheetRequest(sheet, sheetIndex, range, maxRows, maxColumns);
  }

  /**
   * This request with the sheet's index from 0, hidden sheets and chartsheets counted; not together
   * with {@link #sheet(String)}.
   *
   * @param sheetIndex the index; null to give none
   * @return the new request
   */
  public SheetRequest sheetIndex(Integer sheetIndex) {
    return new SheetRequest(sheet, sheetIndex, range, maxRows, maxColumns);
  }

  /**
   * This request with an A1 range such as {@code "A1:H50"}: the cells shown start at its top left
   * cell (A1 without it) and never go past it.
   *
   * @param range the range; null to give none
   * @return the new request
   */
  public SheetRequest range(String range) {
    return new SheetRequest(sheet, sheetIndex, range, maxRows, maxColumns);
  }

  /**
   * This request with the most rows shown.
   *
   * @param maxRows 1 to 10000 (100 without it); null to give none
   * @return the new request
   */
  public SheetRequest maxRows(Integer maxRows) {
    return new SheetRequest(sheet, sheetIndex, range, maxRows, maxColumns);
  }

  /**
   * This request with the most columns shown.
   *
   * @param maxColumns 1 to 1000 (30 without it); null to give none
   * @return the new request
   */
  public SheetRequest maxColumns(Integer maxColumns) {
    return new SheetRequest(sheet, sheetIndex, range, maxRows, maxColumns);
  }

  /**
   * The request the way the engine names its options: the values given, in the shape of its JSON.
   */
  Map<String, Object> json() {
    Map<String, Object> options = new LinkedHashMap<>();
    put(options, "sheet", sheet);
    put(options, "sheetIndex", sheetIndex);
    put(options, "range", range);
    put(options, "maxRows", maxRows);
    put(options, "maxColumns", maxColumns);
    return options;
  }

  private static void put(Map<String, Object> options, String name, Object value) {
    if (value != null) {
      options.put(name, value);
    }
  }
}
