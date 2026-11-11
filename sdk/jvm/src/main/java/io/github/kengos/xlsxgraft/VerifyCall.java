// The C ABI's verify functions (engine/capi/include/xlsx_graft.h, § VERIFY).
// Each method makes the verify handle, copies its result's JSON out and
// closes the handle, so every handle is closed before the method returns
// (docs/sdk.md § Verifying through the C ABI). The handle is written into a
// PointerByReference made before the call and local to the method, so a
// handle the library made is closed even when an exception is thrown after
// the call and before the close.
//
// Every input is a byte[] of the caller's own, which JNA passes through JNI for the call (JNI may
// copy or pin it).
//
// It holds nothing that changes, so any thread may use one at once. A test
// gives it a java.lang.reflect.Proxy of XlsxGraftLibrary that returns a status
// no workbook makes the engine return (PANIC, an unknown status) or that
// writes no handle.

package io.github.kengos.xlsxgraft;

import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;

/** The verify functions of one engine library. */
final class VerifyCall {

  /** The functions this class calls, which the library must export. */
  static final String[] FUNCTIONS = {
    "xlsx_graft_verify_path",
    "xlsx_graft_verify_bytes",
    "xlsx_graft_policy_check",
    "xlsx_graft_verify_result",
    "xlsx_graft_verify_close",
  };

  private final XlsxGraftLibrary api;

  /**
   * The verify functions {@code library} exports.
   *
   * @throws LibraryNotFoundException the library lacks one of them
   */
  VerifyCall(Library library) {
    library.require(FUNCTIONS);
    this.api = library.api();
  }

  /** The functions of {@code api}, for a test that gives a Proxy of XlsxGraftLibrary. */
  VerifyCall(XlsxGraftLibrary api) {
    this.api = api;
  }

  /** The status and the result's JSON for the workbook at {@code path}, UTF-8. */
  Capi.Lent path(byte[] path, byte[] options) {
    return made(
        out ->
            api.xlsx_graft_verify_path(
                path, new SizeT(path.length), options, new SizeT(options.length), out));
  }

  /** The status and the result's JSON for the workbook in {@code data}. */
  Capi.Lent bytes(byte[] data, byte[] options) {
    return made(
        out ->
            api.xlsx_graft_verify_bytes(
                data, new SizeT(data.length), options, new SizeT(options.length), out));
  }

  /** The status and the result's JSON for the policy's JSON, not wrapped in {@code options}. */
  Capi.Lent policy(byte[] policy) {
    return made(out -> api.xlsx_graft_policy_check(policy, new SizeT(policy.length), out));
  }

  /** A function that makes a verify handle, writing it to {@code out}, and returns its status. */
  private interface Maker {
    int make(PointerByReference out);
  }

  // Calls `maker`, copies out the result of the handle it wrote and closes
  // the handle. The engine writes no handle for a status other than OK, so
  // `out` stays null and nothing is closed.
  private Capi.Lent made(Maker maker) {
    PointerByReference out = new PointerByReference();
    try {
      int status = maker.make(out);
      return status == StatusDiagnostic.OK
          ? lent(out.getValue())
          : new Capi.Lent(status, new byte[0]);
    } finally {
      Pointer handle = out.getValue();
      if (handle != null) {
        api.xlsx_graft_verify_close(handle);
      }
    }
  }

  // The status of verify_result, and the handle's JSON copied out (empty
  // when the status is not OK).
  private Capi.Lent lent(Pointer handle) {
    PointerByReference address = new PointerByReference();
    SizeTByReference length = new SizeTByReference();
    int status = api.xlsx_graft_verify_result(handle, address, length);
    if (status != StatusDiagnostic.OK || length.getValue() == 0) {
      return new Capi.Lent(status, new byte[0]);
    }
    return new Capi.Lent(
        status, address.getValue().getByteArray(0, Math.toIntExact(length.getValue())));
  }
}
