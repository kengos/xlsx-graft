package io.github.kengos.xlsxgraft;

/** One cell of a grid that holds a value. */
public final class GridCell {

  private final String ref;
  private final String type;
  private final Object value;
  private final boolean truncated;

  GridCell(String ref, String type, Object value, boolean truncated) {
    this.ref = ref;
    this.type = type;
    this.value = value;
    this.truncated = truncated;
  }

  /**
   * The cell's reference.
   *
   * @return the reference ({@code B3})
   */
  public String ref() {
    return ref;
  }

  /**
   * The cell's type: {@code string}, {@code number}, {@code boolean}, {@code date}, {@code time},
   * {@code datetime}, {@code elapsed} or {@code error}.
   *
   * @return the type
   */
  public String type() {
    return type;
  }

  /**
   * The JSON's value.
   *
   * @return a {@link String} (also for a date, a time, a date-time and an error), a {@link
   *     Boolean}, a {@link Long} for a number written as an integer (an elapsed time's
   *     milliseconds) or a {@link Double} for any other number
   */
  public Object value() {
    return value;
  }

  /**
   * Whether a long text was cut for display.
   *
   * @return true when it was
   */
  public boolean truncated() {
    return truncated;
  }
}
