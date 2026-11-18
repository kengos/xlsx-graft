// The C ABI's view functions (engine/capi/include/xlsx_graft.h, § VIEWS).
// Each method makes the view handle, copies its result's JSON out and closes
// the handle before it returns, so no handle outlives the call
// (docs/sdk.md § Viewing through the C ABI). The handle is written into a
// PointerByReference made before the call and local to the method, so a
// handle the library made is closed even when an exception is thrown after
// the call and before the close, as in VerifyCall.
//
// Every input is a byte[] of the caller's own, which JNA passes through JNI for the call (JNI may
// copy or pin it).
//
// For XLSX_GRAFT_ERR_REQUEST_INVALID the method reads xlsx_graft_error_message right after the
// call, as Capi does for IO: between the call and the read of the message there is nothing that
// could block. The message belongs to the operating system thread, and a virtual thread that
// blocks may come back on another OS thread (on a java.util.concurrent lock since Java 21, on a
// monitor since Java 24). So the function is resolved beforehand and called as a Function, and
// the memory it writes to (messageOut) is allocated before the call (an allocation registers with
// JNA's Cleaner, which locks). The memory is made for each call, not kept in a field as Capi keeps
// its own, because a Viewer is shared by threads.
//
// A ViewCall holds nothing that changes, so any thread may use one at once. A test gives it a
// java.lang.reflect.Proxy of XlsxGraftLibrary that returns a status no workbook makes the engine
// return (PANIC, an unknown status) or that writes no handle.

package io.github.kengos.xlsxgraft;

import com.sun.jna.Function;
import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;

/** The view functions of one engine library. */
final class ViewCall {

  /** The functions this class calls, which the library must export. */
  static final String[] FUNCTIONS = {
    "xlsx_graft_tree_path",
    "xlsx_graft_tree_bytes",
    "xlsx_graft_describe_sheet_path",
    "xlsx_graft_describe_sheet_bytes",
    "xlsx_graft_view_result",
    "xlsx_graft_view_close",
    "xlsx_graft_error_message",
  };

  private static final byte[] NO_OPTIONS = new byte[0];

  private final XlsxGraftLibrary api;
  private final Function errorMessage;

  /**
   * The view functions {@code library} exports.
   *
   * @throws LibraryNotFoundException the library lacks one of them
   */
  ViewCall(Library library) {
    library.require(FUNCTIONS);
    this.api = library.api();
    this.errorMessage = library.function("xlsx_graft_error_message");
  }

  /** The functions of {@code api} and {@code errorMessage}, for a test that gives a Proxy. */
  ViewCall(XlsxGraftLibrary api, Function errorMessage) {
    this.api = api;
    this.errorMessage = errorMessage;
  }

  /** The parts of the workbook at {@code path}, UTF-8; a tree takes no options. */
  Outcome treePath(byte[] path) {
    return made(
        out ->
            api.xlsx_graft_tree_path(path, new SizeT(path.length), NO_OPTIONS, new SizeT(0), out));
  }

  /** The parts of the workbook in {@code data}, which the call reads while it runs. */
  Outcome treeBytes(byte[] data) {
    return made(
        out ->
            api.xlsx_graft_tree_bytes(data, new SizeT(data.length), NO_OPTIONS, new SizeT(0), out));
  }

  /**
   * The cells of a sheet of the workbook at {@code path}, UTF-8, as the JSON {@code options} says.
   */
  Outcome sheetPath(byte[] path, byte[] options) {
    return made(
        out ->
            api.xlsx_graft_describe_sheet_path(
                path, new SizeT(path.length), options, new SizeT(options.length), out));
  }

  /** The cells of a sheet of the workbook in {@code data}, as the JSON {@code options} says. */
  Outcome sheetBytes(byte[] data, byte[] options) {
    return made(
        out ->
            api.xlsx_graft_describe_sheet_bytes(
                data, new SizeT(data.length), options, new SizeT(options.length), out));
  }

  /** A function that makes a view handle, writing it to {@code out}, and returns its status. */
  private interface Maker {
    int make(PointerByReference out);
  }

  // Calls `maker`, copies out the result of the handle it wrote and closes
  // the handle. The engine writes no handle for a status other than OK, so
  // `out` stays null and nothing is closed.
  private Outcome made(Maker maker) {
    Memory messageOut = new Memory(Native.POINTER_SIZE + Native.SIZE_T_SIZE);
    Pointer length = messageOut.share(Native.POINTER_SIZE);
    PointerByReference out = new PointerByReference();
    try {
      int status = maker.make(out);
      if (status == StatusDiagnostic.OK) {
        return copied(out.getValue());
      }
      if (status == StatusDiagnostic.REQUEST_INVALID) {
        return new Outcome(status, null, message(messageOut, length));
      }
      return new Outcome(status, null, null);
    } finally {
      Pointer handle = out.getValue();
      if (handle != null) {
        api.xlsx_graft_view_close(handle);
      }
    }
  }

  // The status of view_result, and the handle's JSON copied out (null when
  // the status is not OK, empty when it has no bytes).
  private Outcome copied(Pointer handle) {
    PointerByReference address = new PointerByReference();
    SizeTByReference length = new SizeTByReference();
    int status = api.xlsx_graft_view_result(handle, address, length);
    if (status != StatusDiagnostic.OK) {
      return new Outcome(status, null, null);
    }
    byte[] json =
        length.getValue() == 0
            ? new byte[0]
            : address.getValue().getByteArray(0, Math.toIntExact(length.getValue()));
    return new Outcome(status, json, null);
  }

  // Why the call that returned REQUEST_INVALID failed, read right now; null when
  // xlsx_graft_error_message does not return OK.
  private String message(Memory messageOut, Pointer length) {
    int read = errorMessage.invokeInt(new Object[] {messageOut, length});
    return read == StatusDiagnostic.OK ? written(messageOut, length) : null;
  }

  private static String written(Memory messageOut, Pointer length) {
    return Capi.text(messageOut.getPointer(0), Capi.sizeT(length, 0));
  }

  /** What a view function returned. */
  static final class Outcome {

    final int status;

    /** The view's JSON, copied out; null for a status other than OK. */
    final byte[] json;

    /**
     * The engine's message for REQUEST_INVALID; null when it could not be read, or for any other
     * status.
     */
    final String message;

    Outcome(int status, byte[] json, String message) {
      this.status = status;
      this.json = json;
      this.message = message;
    }
  }
}
