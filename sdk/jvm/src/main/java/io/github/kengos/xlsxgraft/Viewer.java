package io.github.kengos.xlsxgraft;

import java.nio.file.Path;
import java.util.Map;
import java.util.Objects;

/**
 * Shows the parts of a workbook and the cells of its sheets through the C ABI: the engine's {@code
 * tree} and {@code describe_sheet} views. The limits a workbook is read under are the CLI's
 * defaults; a viewer takes no policy.
 *
 * <p>A viewer does not change once it is made, so one may be shared by any number of threads, and
 * their calls may run at once.
 *
 * <pre>{@code
 * Viewer viewer = new Viewer();
 * Tree tree = viewer.tree(Paths.get("book.xlsx")).value();
 * if (tree != null) {
 *   for (TreePart part : tree.parts()) {
 *     System.out.println(part.name() + " " + part.contentType());
 *   }
 * }
 *
 * SheetRequest request = SheetRequest.first().sheet("Data").range("A1:H50").maxRows(20);
 * Result<Grid> result = viewer.describeSheet(Paths.get("book.xlsx"), request);
 * Grid grid = result.value();
 * if (grid != null) {
 *   for (GridRow row : grid.rows()) {
 *     for (GridCell cell : row.cells()) {
 *       System.out.println(cell.ref() + " " + cell.value());
 *     }
 *   }
 * }
 * if (result.error() != null) {
 *   System.err.println(result.error().message());
 * }
 * }</pre>
 *
 * <p>The result of a view is a {@code Result<Tree>} or a {@code Result<Grid>}: it succeeds when the
 * engine's {@code ok} is true, and its value is there also when it is not (a workbook that is
 * broken, or a file that cannot be opened, gives a tree with the parts that were read, perhaps
 * none; a sheet that is not found gives a grid whose {@link Grid#sheet()} is null). The value is
 * null only when the package failed to get one: {@link Result#error()} is {@code panic}, {@code
 * null_arg}, {@code unknown_status}, {@code diagnostics_unreadable}, or {@code io} for a path that
 * is not Unicode whose file the package cannot read. A file that cannot be opened is an {@code io}
 * {@link Result#error()}, with a value.
 */
public final class Viewer {

  private final ViewCall call;

  /** A viewer over the engine library the package finds. */
  public Viewer() {
    this((String) null);
  }

  /**
   * A viewer over the engine library at {@code library}.
   *
   * @param library the engine library's path, used when {@code XLSX_GRAFT_LIBRARY} is not set; may
   *     be null
   * @throws LibraryNotFoundException no engine library was found or loaded, or it lacks a function
   */
  public Viewer(String library) {
    this(library, System.getenv());
  }

  Viewer(String library, Map<String, String> env) {
    this(new ViewCall(Library.open(library, env)));
  }

  // The view functions given, for a test that answers in place of the engine.
  Viewer(ViewCall call) {
    this.call = call;
  }

  /**
   * Shows the parts of the workbook at {@code path} and the relationships between them.
   *
   * <p>A path holding an unpaired surrogate, which only a Windows file name can hold and which
   * cannot be encoded as UTF-8, is not given to the engine: the package reads the file and shows
   * its bytes, so a file it cannot read is an {@code io} error with no value.
   *
   * @param path the workbook's path, a relative one taken from the working directory as Java takes
   *     it ({@code user.dir})
   * @return the result (see the class description)
   * @throws NullPointerException {@code path} is null
   */
  public Result<Tree> tree(Path path) {
    Objects.requireNonNull(path, "path");
    return PathInput.call(
        path,
        encoded -> ViewReport.call(call.treePath(encoded), ViewContent::tree),
        this::treeBytes);
  }

  /**
   * As {@link #tree(Path)}, for the workbook in {@code data}.
   *
   * <p>The package does not copy {@code data}: the call reads the array while it runs (JNA passes
   * it through JNI, which may copy or pin it), so do not change the array until the call returns.
   *
   * @param data the workbook's bytes
   * @return the result (see the class description)
   * @throws NullPointerException {@code data} is null
   */
  public Result<Tree> treeBytes(byte[] data) {
    Objects.requireNonNull(data, "data");
    return ViewReport.call(call.treeBytes(data), ViewContent::tree);
  }

  /**
   * Shows the cells of the first sheet of the workbook at {@code path}, in the engine's default
   * window.
   *
   * @param path the workbook's path; as for {@link #tree(Path)}
   * @return the result (see the class description)
   * @throws NullPointerException {@code path} is null
   */
  public Result<Grid> describeSheet(Path path) {
    return describeSheet(path, SheetRequest.first());
  }

  /**
   * Shows the cells of the sheet {@code request} names, in the window it gives.
   *
   * @param path the workbook's path; as for {@link #tree(Path)}
   * @param request which sheet and how much of it
   * @return the result (see the class description)
   * @throws NullPointerException {@code path} or {@code request} is null
   * @throws IllegalArgumentException the engine refuses a value of {@code request}, with its
   *     message (a range that is no A1 range, {@code maxRows} out of range, a name together with an
   *     index, an index below 0); or, before the call, the name or the range holds text that cannot
   *     be encoded as UTF-8 (an unpaired surrogate), or the options are more than 2 MiB of JSON
   */
  public Result<Grid> describeSheet(Path path, SheetRequest request) {
    Objects.requireNonNull(path, "path");
    byte[] options = options(request);
    return PathInput.call(
        path,
        encoded -> ViewReport.call(call.sheetPath(encoded, options), ViewContent::grid),
        data -> ViewReport.call(call.sheetBytes(data, options), ViewContent::grid));
  }

  /**
   * As {@link #describeSheet(Path)}, for the workbook in {@code data}, which is not copied (see
   * {@link #treeBytes(byte[])}).
   *
   * @param data the workbook's bytes
   * @return the result (see the class description)
   * @throws NullPointerException {@code data} is null
   */
  public Result<Grid> describeSheetBytes(byte[] data) {
    return describeSheetBytes(data, SheetRequest.first());
  }

  /**
   * As {@link #describeSheet(Path, SheetRequest)}, for the workbook in {@code data}, which is not
   * copied (see {@link #treeBytes(byte[])}).
   *
   * @param data the workbook's bytes
   * @param request which sheet and how much of it
   * @return the result (see the class description)
   * @throws NullPointerException {@code data} or {@code request} is null
   * @throws IllegalArgumentException as for {@link #describeSheet(Path, SheetRequest)}
   */
  public Result<Grid> describeSheetBytes(byte[] data, SheetRequest request) {
    Objects.requireNonNull(data, "data");
    return ViewReport.call(call.sheetBytes(data, options(request)), ViewContent::grid);
  }

  // The options JSON as UTF-8: the request's values by the engine's names.
  private static byte[] options(SheetRequest request) {
    Objects.requireNonNull(request, "request");
    byte[] options = Capi.utf8(JsonWriter.write(request.json()));
    if (options == null) {
      throw new IllegalArgumentException(
          "the sheet's name or the range holds text that cannot be encoded as UTF-8 (an unpaired surrogate)");
    }
    if (options.length > Verifier.MAX_OPTIONS_BYTES) {
      throw new IllegalArgumentException(
          "the options are "
              + options.length
              + " bytes of JSON; the engine takes "
              + Verifier.MAX_OPTIONS_BYTES);
    }
    return options;
  }
}
