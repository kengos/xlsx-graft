package io.github.kengos.xlsxgraft;

import com.sun.jna.ptr.PointerByReference;

/** A sheet asked for by name or index, from {@link Book#sheet(String)}. */
public final class Sheet {

  private final Book book;
  private final String name;
  private final int index;

  Sheet(Book book, String name, int index) {
    this.book = book;
    this.name = name;
    this.index = index;
  }

  /**
   * The rows the sheet writes, read from its start, one at a time as they are iterated: no more of
   * the sheet is held than the row given. Only the rows the sheet writes come.
   *
   * <p>Reading stops at the first problem in the sheet, the rows before it having come, and gives
   * nothing when the engine refuses the sheet (no such sheet, not a worksheet); either way the
   * book's {@link Book#result()} fails, saying why.
   *
   * <p>The rows hold what the engine keeps of the sheet until they are read to the end or closed,
   * or the book is: read them in a try-with-resources block of their own to free them sooner.
   *
   * @return the rows, to iterate once
   * @throws IllegalStateException the book is closed
   */
  public Rows rows() {
    PointerByReference slot = book.openRows(name, index);
    return new Rows(book, slot);
  }
}
