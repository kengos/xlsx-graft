package io.github.kengos.xlsxgraft;

import java.util.Collections;
import java.util.List;

/**
 * One row a sheet writes: its number, from 1, and the value of each column from A to its last cell,
 * null where a column has none.
 *
 * <p>A value is null, a {@link String}, a {@link Double}, a {@link Boolean}, a {@link
 * java.time.LocalDate}, a {@link java.time.LocalTime}, a {@link java.time.LocalDateTime} (the
 * workbook's wall-clock time: no time zone is applied), a {@link java.time.Duration} (an elapsed
 * time, not wrapped at a day) or a {@link CellError}.
 *
 * <p>Two rows are equal when their numbers are and their values are, one by one.
 */
public final class Row {

  private final int number;
  private final List<Object> values;

  Row(int number, List<Object> values) {
    this.number = number;
    this.values = Collections.unmodifiableList(values);
  }

  /**
   * The row's number.
   *
   * @return from 1 to 1,048,576
   */
  public int number() {
    return number;
  }

  /**
   * The value of each column from A.
   *
   * @return the values, which cannot be changed
   */
  public List<Object> values() {
    return values;
  }

  @Override
  public boolean equals(Object other) {
    if (!(other instanceof Row)) {
      return false;
    }
    Row that = (Row) other;
    return number == that.number && values.equals(that.values);
  }

  @Override
  public int hashCode() {
    return 31 * number + values.hashCode();
  }

  @Override
  public String toString() {
    return number + ": " + values;
  }
}
