// Finds and opens the engine's shared library (libxlsx_graft_capi).
//
// Looked up in this order, and the first one set wins (a set path that does
// not load throws rather than falls through):
//
// 1. the XLSX_GRAFT_LIBRARY environment variable
// 2. the caller's path
// 3. a library in native/ on the classpath
//
// The environment comes first because where the engine lives is the
// deployment's decision, and it has to be able to override application code.
// Nothing here downloads anything.

package io.github.kengos.xlsxgraft;

import com.sun.jna.Native;
import java.io.File;
import java.io.IOException;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.WeakHashMap;
import java.util.concurrent.ConcurrentHashMap;

/** One opened engine library. */
final class Library {

  static final String ENV_VAR = "XLSX_GRAFT_LIBRARY";

  /** The file name on each platform. Windows has no {@code lib} prefix. */
  static final List<String> NAMES =
      Collections.unmodifiableList(
          Arrays.asList(
              "libxlsx_graft_capi.so", "libxlsx_graft_capi.dylib", "xlsx_graft_capi.dll"));

  // The packaged copy found through each class loader. Out of a jar, JNA
  // extracts a new temporary file on every call, which would load one more
  // copy of the engine each time.
  private static final Map<ClassLoader, String> PACKAGED =
      Collections.synchronizedMap(new WeakHashMap<ClassLoader, String>());

  // Every library opened, by its absolute path, held for the life of the
  // process. JNA closes a library its binding no longer reaches when the
  // garbage collector clears it, and unloading a Rust library that has run
  // thread-local destructors is not safe.
  private static final Map<String, XlsxGraftLibrary> OPENED = new ConcurrentHashMap<>();

  private final String path;
  private final XlsxGraftLibrary api;

  private Library(String path, XlsxGraftLibrary api) {
    this.path = path;
    this.api = api;
  }

  /** The file that was opened. */
  String path() {
    return path;
  }

  /** The library's functions. */
  XlsxGraftLibrary api() {
    return api;
  }

  /** The library {@code env} or {@code path} names, or the packaged copy on the classpath. */
  static Library open(String path, Map<String, String> env) {
    return open(path, env, Library.class.getClassLoader());
  }

  /**
   * The library {@code env} or {@code path} names, or the packaged copy {@code loader} holds.
   *
   * @throws LibraryNotFoundException no library was found, or it did not load
   */
  static Library open(String path, Map<String, String> env, ClassLoader loader) {
    // An empty value counts as unset, as it does in every SDK.
    String found = nonEmpty(env.get(ENV_VAR));
    if (found == null) {
      found = nonEmpty(path);
    }
    if (found == null) {
      found = PACKAGED.computeIfAbsent(loader, Library::packaged);
    }
    if (found == null) {
      throw new LibraryNotFoundException(installHint("no engine library was found"));
    }
    return new Library(found, load(found));
  }

  private static String nonEmpty(String value) {
    return value == null || value.isEmpty() ? null : value;
  }

  // Looked for as a resource, not as a directory: inside a jar, native/ has
  // no path on disk. JNA hands back the file itself when the resource is one,
  // and a copy in a temporary directory of its own when it is in a jar.
  private static String packaged(ClassLoader loader) {
    for (String name : NAMES) {
      try {
        return Native.extractFromResourcePath("/native/" + name, loader).getAbsolutePath();
      } catch (IOException absent) {
        // Not this platform's file name; try the next one.
      }
    }
    return null;
  }

  // Handed the absolute path, so JNA does not look the name up on the library
  // path. Never closed (OPENED).
  private static XlsxGraftLibrary load(String path) {
    try {
      return OPENED.computeIfAbsent(
          new File(path).getAbsolutePath(), file -> Native.load(file, XlsxGraftLibrary.class));
    } catch (UnsatisfiedLinkError error) {
      throw new LibraryNotFoundException(
          installHint(path + " could not be loaded (" + error.getMessage() + ")"), error);
    }
  }

  private static String installHint(String reason) {
    return reason
        + ".\n\n"
        + "This package never downloads the engine. Point it at one of these ways:\n"
        + "  * set "
        + ENV_VAR
        + " to the path of a libxlsx_graft_capi library\n"
        + "  * pass Engine.version(\"/path/to/libxlsx_graft_capi.so\")\n"
        + "  * put the library in native/ on the classpath";
  }
}
