package io.github.kengos.xlsxgraft;

import java.nio.file.Path;
import java.util.Map;

/**
 * Opens a workbook to read its sheets' rows, through the C ABI.
 *
 * <pre>{@code
 * Book book = XlsxReader.open(Paths.get("book.xlsx"));
 * try (Book open = book) {
 *   for (Row row : open.sheet("Data").rows()) {
 *     System.out.println(row.number() + ": " + row.values());
 *   }
 * }
 *
 * // after the block
 * Result<Void> result = book.result();
 * }</pre>
 */
public final class XlsxReader {

  private XlsxReader() {}

  /**
   * The workbook at {@code path}, opened now.
   *
   * <p>A file that cannot be opened gives a book all the same, with no sheets, whose {@link
   * Book#result()} says why ({@code io}). A path that is not Unicode (a lone surrogate) is refused
   * with {@code invalid_arg}.
   *
   * @param path the workbook's path, a relative one taken from the working directory as Java takes
   *     it ({@code user.dir})
   * @return the book, to read and close
   * @throws LibraryNotFoundException no engine library was found or loaded, or it lacks a function
   */
  public static Book open(Path path) {
    return open(path, null);
  }

  /**
   * As {@link #open(Path)}, from the engine library at {@code library}.
   *
   * @param path the workbook's path
   * @param library the engine library's path, used when {@code XLSX_GRAFT_LIBRARY} is not set; may
   *     be null
   * @return the book, to read and close
   * @throws LibraryNotFoundException no engine library was found or loaded, or it lacks a function
   */
  public static Book open(Path path, String library) {
    return open(path, library, System.getenv());
  }

  /**
   * As {@link #open(Path)}, the workbook being the bytes of {@code data}, copied now: changing them
   * after changes nothing.
   *
   * @param data the workbook's bytes
   * @return the book, to read and close
   * @throws LibraryNotFoundException no engine library was found or loaded, or it lacks a function
   */
  public static Book openBytes(byte[] data) {
    return openBytes(data, null);
  }

  /**
   * As {@link #openBytes(byte[])}, from the engine library at {@code library}.
   *
   * @param data the workbook's bytes
   * @param library the engine library's path, used when {@code XLSX_GRAFT_LIBRARY} is not set; may
   *     be null
   * @return the book, to read and close
   * @throws LibraryNotFoundException no engine library was found or loaded, or it lacks a function
   */
  public static Book openBytes(byte[] data, String library) {
    return openBytes(data, library, System.getenv());
  }

  static Book open(Path path, String library, Map<String, String> env) {
    return open(path.toAbsolutePath().toString(), library, env);
  }

  // The path as text: a lone surrogate makes no Path on Linux or macOS, but
  // one does on Windows, whose file names are UTF-16.
  static Book open(String path, String library, Map<String, String> env) {
    Capi capi = new Capi(Library.open(library, env));
    byte[] encoded = Capi.utf8(path);
    return encoded == null
        ? new Book(capi, (c, book) -> StatusDiagnostic.INVALID_ARG)
        : new Book(capi, (c, book) -> c.bookOpenPath(encoded, book));
  }

  static Book openBytes(byte[] data, String library, Map<String, String> env) {
    byte[] copy = data.clone();
    Capi capi = new Capi(Library.open(library, env));
    return new Book(capi, (c, book) -> c.bookOpenBytes(copy, book));
  }
}
