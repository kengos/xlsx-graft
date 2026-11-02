package io.github.kengos.xlsxgraft;

/**
 * A sheet as the workbook lists it, from {@link Book#sheets()}. Two are equal when their parts are.
 */
public final class SheetInfo {

  private final int index;
  private final String name;
  private final SheetKind kind;
  private final SheetState state;

  SheetInfo(int index, String name, SheetKind kind, SheetState state) {
    this.index = index;
    this.name = name;
    this.kind = kind;
    this.state = state;
  }

  /**
   * Where the sheet is in the workbook's list, as {@link Book#sheet(int)} counts.
   *
   * @return from 0, hidden sheets and chartsheets counted
   */
  public int index() {
    return index;
  }

  /**
   * The sheet's name.
   *
   * @return the name
   */
  public String name() {
    return name;
  }

  /**
   * What the sheet is; only a worksheet has rows to read.
   *
   * @return the kind
   */
  public SheetKind kind() {
    return kind;
  }

  /**
   * Whether it is shown.
   *
   * @return the state
   */
  public SheetState state() {
    return state;
  }

  @Override
  public boolean equals(Object other) {
    if (!(other instanceof SheetInfo)) {
      return false;
    }
    SheetInfo that = (SheetInfo) other;
    return index == that.index
        && name.equals(that.name)
        && kind == that.kind
        && state == that.state;
  }

  @Override
  public int hashCode() {
    return 31 * index + name.hashCode();
  }

  @Override
  public String toString() {
    return index + " " + name + " (" + kind + ", " + state + ")";
  }
}
