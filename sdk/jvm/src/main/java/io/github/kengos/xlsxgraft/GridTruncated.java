package io.github.kengos.xlsxgraft;

/** Which of a grid's limits cut what it shows. */
public final class GridTruncated {

  private final boolean rows;
  private final boolean columns;
  private final boolean merges;

  GridTruncated(boolean rows, boolean columns, boolean merges) {
    this.rows = rows;
    this.columns = columns;
    this.merges = merges;
  }

  /**
   * Whether rows were left out.
   *
   * @return true when they were
   */
  public boolean rows() {
    return rows;
  }

  /**
   * Whether columns were left out.
   *
   * @return true when they were
   */
  public boolean columns() {
    return columns;
  }

  /**
   * Whether merged ranges were left out.
   *
   * @return true when they were
   */
  public boolean merges() {
    return merges;
  }
}
