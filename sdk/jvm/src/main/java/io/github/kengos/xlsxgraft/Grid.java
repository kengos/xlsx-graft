package io.github.kengos.xlsxgraft;

import java.util.List;

/**
 * The cells of one sheet, the value of {@link Viewer#describeSheet(java.nio.file.Path)}. Names and
 * text are made for display (control characters removed, long text cut), so they are not exact;
 * read the rows with {@link XlsxReader} for exact values.
 */
public final class Grid {

  private final SheetInfo sheet;
  private final String dimension;
  private final String used;
  private final String shown;
  private final GridTruncated truncated;
  private final boolean complete;
  private final List<GridRow> rows;
  private final List<String> merges;

  Grid(
      SheetInfo sheet,
      String dimension,
      String used,
      String shown,
      GridTruncated truncated,
      boolean complete,
      List<GridRow> rows,
      List<String> merges) {
    this.sheet = sheet;
    this.dimension = dimension;
    this.used = used;
    this.shown = shown;
    this.truncated = truncated;
    this.complete = complete;
    this.rows = rows;
    this.merges = merges;
  }

  /**
   * The sheet shown.
   *
   * @return the sheet, or null when none was found
   */
  public SheetInfo sheet() {
    return sheet;
  }

  /**
   * The range the sheet declares.
   *
   * @return the range, or null when it declares none
   */
  public String dimension() {
    return dimension;
  }

  /**
   * The range that holds cells.
   *
   * @return the range, or null when the sheet is empty
   */
  public String used() {
    return used;
  }

  /**
   * The range of the cells shown.
   *
   * @return the range, or null when none are
   */
  public String shown() {
    return shown;
  }

  /**
   * Which limits cut what is shown.
   *
   * @return the limits
   */
  public GridTruncated truncated() {
    return truncated;
  }

  /**
   * Whether the whole sheet was read.
   *
   * @return true when it was
   */
  public boolean complete() {
    return complete;
  }

  /**
   * The rows shown.
   *
   * @return the rows, which cannot be changed
   */
  public List<GridRow> rows() {
    return rows;
  }

  /**
   * The merged ranges ({@code A1:C1}).
   *
   * @return the ranges, which cannot be changed
   */
  public List<String> merges() {
    return merges;
  }
}
