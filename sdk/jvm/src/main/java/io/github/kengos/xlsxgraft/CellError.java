package io.github.kengos.xlsxgraft;

/**
 * A cell holding an error value, its text as the workbook writes it ({@code "#N/A"}, {@code
 * "#DIV/0!"}). Kept apart from a {@link String} so that a cell holding the text {@code "#N/A"} is
 * not read as the error. Two are equal when their texts are.
 */
public final class CellError {

  private final String text;

  CellError(String text) {
    this.text = text;
  }

  /**
   * The error's text.
   *
   * @return the text, such as {@code "#N/A"}
   */
  public String text() {
    return text;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof CellError && text.equals(((CellError) other).text);
  }

  @Override
  public int hashCode() {
    return text.hashCode();
  }

  @Override
  public String toString() {
    return text;
  }
}
