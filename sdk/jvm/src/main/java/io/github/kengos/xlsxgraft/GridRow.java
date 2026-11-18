package io.github.kengos.xlsxgraft;

import java.util.List;

/** One row of a grid. */
public final class GridRow {

  private final int row;
  private final List<GridCell> cells;

  GridRow(int row, List<GridCell> cells) {
    this.row = row;
    this.cells = cells;
  }

  /**
   * The row's number, from 1.
   *
   * @return the number
   */
  public int row() {
    return row;
  }

  /**
   * The row's cells that hold a value, from the left.
   *
   * @return the cells, which cannot be changed
   */
  public List<GridCell> cells() {
    return cells;
  }
}
