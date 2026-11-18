// Gives a workbook's path to a call that takes a path or the workbook's bytes.

package io.github.kengos.xlsxgraft;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Collections;
import java.util.function.Function;

final class PathInput {

  private PathInput() {}

  /**
   * The result of {@code byPath} for {@code path} as the engine takes it (an absolute path encoded
   * as UTF-8), or, when the path cannot be encoded as UTF-8, that of {@code byBytes} for the bytes
   * of the file (see {@link #read}).
   */
  static <T> Result<T> call(
      Path path, Function<byte[], Result<T>> byPath, Function<byte[], Result<T>> byBytes) {
    byte[] encoded = Capi.utf8(path.toAbsolutePath().toString());
    return encoded == null ? read(path, byBytes) : byPath.apply(encoded);
  }

  // The engine takes no path that cannot be encoded as UTF-8, so the package
  // reads the file and gives its bytes; a file that cannot be read is an `io` failure.
  static <T> Result<T> read(Path path, Function<byte[], Result<T>> byBytes) {
    byte[] data;
    try {
      data = Files.readAllBytes(path);
    } catch (IOException unreadable) {
      return Result.failure(
          Collections.<Diagnostic>emptyList(),
          StatusDiagnostic.error("io", "the file could not be read: " + reason(unreadable)));
    }
    return byBytes.apply(data);
  }

  // The message of `failure`, or its class name when it has none.
  static String reason(IOException failure) {
    return failure.getMessage() == null ? failure.getClass().getName() : failure.getMessage();
  }
}
