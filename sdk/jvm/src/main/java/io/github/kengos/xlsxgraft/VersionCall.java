// Calls xlsx_graft_version and returns the engine's version as a Result.

package io.github.kengos.xlsxgraft;

import com.sun.jna.ptr.PointerByReference;
import java.nio.charset.StandardCharsets;

final class VersionCall {

  static final String NAME = "xlsx_graft_version";

  private VersionCall() {}

  /**
   * The version string, such as {@code "0.0.0"}, on success.
   *
   * @param api the library's functions
   * @param path the library's file, for the message when it exports no {@code xlsx_graft_version}
   * @throws LibraryNotFoundException the library exports no {@code xlsx_graft_version}
   */
  static Result<String> call(XlsxGraftLibrary api, String path) {
    PointerByReference outPtr = new PointerByReference();
    SizeTByReference outLen = new SizeTByReference();
    int status;
    try {
      status = api.xlsx_graft_version(outPtr, outLen);
    } catch (UnsatisfiedLinkError missing) {
      // JNA looks a function up on its first call, not when the library opens.
      throw new LibraryNotFoundException(path + " exports no `" + NAME + "`", missing);
    }
    if (status != StatusDiagnostic.OK) {
      return Result.failure(StatusDiagnostic.of(status));
    }
    // The bytes are not NUL-terminated, so exactly `outLen` of them are
    // copied out. The header guarantees the length; one past
    // Integer.MAX_VALUE throws ArithmeticException rather than reading out of
    // range.
    byte[] bytes = outPtr.getValue().getByteArray(0, Math.toIntExact(outLen.getValue()));
    return Result.ok(new String(bytes, StandardCharsets.UTF_8));
  }
}
