// The package's entry points.
//
// Every one opens the engine library (Library.java) and calls into the C ABI
// through JNA's interface mapping (XlsxGraftLibrary.java says why not direct
// mapping).

package io.github.kengos.xlsxgraft;

import java.util.Map;

/**
 * Calls into the xlsx-graft engine.
 *
 * <pre>{@code
 * Result<String> result = Engine.version();
 * if (result.success()) {
 *   System.out.println(result.value());
 * }
 * }</pre>
 */
public final class Engine {

  private Engine() {}

  /**
   * The engine's version, such as {@code "0.0.0"}, on success.
   *
   * @return the version, or a failure carrying the engine's diagnostics
   * @throws LibraryNotFoundException no engine library was found or loaded, or the one found
   *     exports no {@code xlsx_graft_version}
   */
  public static Result<String> version() {
    return version(null);
  }

  /**
   * The engine's version, such as {@code "0.0.0"}, from the library at {@code library}.
   *
   * @param library the engine library's path, used when {@code XLSX_GRAFT_LIBRARY} is not set; may
   *     be null
   * @return the version, or a failure carrying the engine's diagnostics
   * @throws LibraryNotFoundException no engine library was found or loaded, or the one found
   *     exports no {@code xlsx_graft_version}
   */
  public static Result<String> version(String library) {
    return version(library, System.getenv());
  }

  static Result<String> version(String library, Map<String, String> env) {
    Library opened = Library.open(library, env);
    return VersionCall.call(opened.api(), opened.path());
  }
}
