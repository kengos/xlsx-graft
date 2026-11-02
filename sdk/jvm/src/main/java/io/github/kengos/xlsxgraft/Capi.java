// The C ABI's book and rows functions (engine/capi/include/xlsx_graft.h).
// Each method makes one call and returns its status, and what it wrote is
// only meaningful when the status is XLSX_GRAFT_OK.
//
// A handle is written into a PointerByReference the caller owns, made before
// the call, so a handle the library made is kept even when an exception comes
// between the call and the next line. What the library lends (sheet names,
// the diagnostics' JSON) is copied out before the method returns. A row's
// values are not: CellValues reads them, before the next call on that rows
// handle, or its close.
//
// Every input is a byte[] of the caller's own, which JNA copies for the call.
//
// xlsx_graft_error_message says why the last call on the operating system's
// thread failed with XLSX_GRAFT_ERR_IO, so it is read right after that call,
// in the same method, nothing between that could block: a virtual thread
// that blocks may come back on another OS thread (on a java.util.concurrent
// lock since Java 21, on a monitor since Java 24). So it is called as a
// Function resolved beforehand, not through the binding's proxy (whose
// method lookup locks), into memory allocated beforehand (an allocation
// registers with JNA's Cleaner, which locks). Loading a class the first
// time is the one wait left.
//
// Thread safety is the caller's: nothing here locks. The class and its
// methods are not final so that a test can make one of them answer what no
// workbook makes the engine answer.

package io.github.kengos.xlsxgraft;

import com.sun.jna.Function;
import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.PointerByReference;
import java.nio.ByteBuffer;
import java.nio.CharBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.List;

/** The functions of one engine library. */
class Capi {

  /** The functions this class calls, which the library must export. */
  static final String[] FUNCTIONS = {
    "xlsx_graft_error_message",
    "xlsx_graft_book_open_path",
    "xlsx_graft_book_open_bytes",
    "xlsx_graft_book_sheets",
    "xlsx_graft_book_diagnostics",
    "xlsx_graft_book_close",
    "xlsx_graft_rows_open",
    "xlsx_graft_rows_open_name",
    "xlsx_graft_rows_next",
    "xlsx_graft_rows_close",
  };

  // XlsxGraftSheet: name, name_len, kind, state. 24 bytes on a 64-bit target.
  static final int SHEET_NAME_LEN = Native.POINTER_SIZE;
  static final int SHEET_KIND = SHEET_NAME_LEN + Native.SIZE_T_SIZE;
  static final int SHEET_STATE = SHEET_KIND + 4;
  static final int SHEET_SIZE = align(SHEET_STATE + 4, Native.POINTER_SIZE);

  private final XlsxGraftLibrary api;
  private final Function errorMessage;

  // Where xlsx_graft_error_message writes: the pointer, then the length.
  private final Memory lent = new Memory(Native.POINTER_SIZE + Native.SIZE_T_SIZE);
  private final Pointer lentLength = lent.share(Native.POINTER_SIZE);

  // Why the last call that returned XLSX_GRAFT_ERR_IO failed.
  private String reason = "";

  /**
   * The functions {@code library} exports.
   *
   * @throws LibraryNotFoundException the library lacks one of them
   */
  Capi(Library library) {
    library.require(FUNCTIONS);
    this.api = library.api();
    this.errorMessage = library.function("xlsx_graft_error_message");
  }

  /** A struct's {@code size} padded to a multiple of its {@code alignment}, as C pads it. */
  static int align(int size, int alignment) {
    return (size + alignment - 1) / alignment * alignment;
  }

  /**
   * The UTF-8 bytes of {@code text}; null for a string that is not Unicode (a lone surrogate),
   * which the engine would refuse.
   */
  static byte[] utf8(String text) {
    try {
      ByteBuffer encoded =
          StandardCharsets.UTF_8
              .newEncoder()
              .onMalformedInput(CodingErrorAction.REPORT)
              .onUnmappableCharacter(CodingErrorAction.REPORT)
              .encode(CharBuffer.wrap(text));
      byte[] bytes = new byte[encoded.remaining()];
      encoded.get(bytes);
      return bytes;
    } catch (CharacterCodingException notUnicode) {
      return null;
    }
  }

  /** The {@code length} UTF-8 bytes at {@code address}, copied out. */
  static String text(Pointer address, long length) {
    // One past Integer.MAX_VALUE throws ArithmeticException rather than
    // reading out of range.
    return length == 0
        ? ""
        : new String(address.getByteArray(0, Math.toIntExact(length)), StandardCharsets.UTF_8);
  }

  /** Writes the book handle into {@code book}. */
  int bookOpenPath(byte[] path, PointerByReference book) {
    return noted(api.xlsx_graft_book_open_path(path, new SizeT(path.length), book));
  }

  /** Writes the book handle into {@code book}. */
  int bookOpenBytes(byte[] data, PointerByReference book) {
    return noted(api.xlsx_graft_book_open_bytes(data, new SizeT(data.length), book));
  }

  /** Adds each sheet to {@code sheets}, in the order the workbook lists them. */
  int bookSheets(Pointer book, List<SheetInfo> sheets) {
    PointerByReference array = new PointerByReference();
    SizeTByReference count = new SizeTByReference();
    int status = noted(api.xlsx_graft_book_sheets(book, array, count));
    if (status != StatusDiagnostic.OK) {
      return status;
    }
    int listed = Math.toIntExact(count.getValue());
    for (int index = 0; index < listed; index++) {
      Pointer sheet = array.getValue().share((long) index * SHEET_SIZE);
      sheets.add(
          new SheetInfo(
              index,
              text(sheet.getPointer(0), sizeT(sheet, SHEET_NAME_LEN)),
              SheetKind.of(sheet.getInt(SHEET_KIND)),
              SheetState.of(sheet.getInt(SHEET_STATE))));
    }
    return status;
  }

  /** The status and the diagnostics' JSON, its bytes copied out. */
  Lent bookDiagnostics(Pointer book) {
    PointerByReference address = new PointerByReference();
    SizeTByReference length = new SizeTByReference();
    int status = noted(api.xlsx_graft_book_diagnostics(book, address, length));
    if (status != StatusDiagnostic.OK || length.getValue() == 0) {
      return new Lent(status, new byte[0]);
    }
    return new Lent(status, address.getValue().getByteArray(0, Math.toIntExact(length.getValue())));
  }

  int bookClose(Pointer book) {
    return noted(api.xlsx_graft_book_close(book));
  }

  /** Writes the rows handle into {@code rows}. */
  int rowsOpen(Pointer book, int index, PointerByReference rows) {
    return noted(api.xlsx_graft_rows_open(book, new SizeT(index), rows));
  }

  /** Writes the rows handle into {@code rows}. */
  int rowsOpenName(Pointer book, byte[] name, PointerByReference rows) {
    return noted(api.xlsx_graft_rows_open_name(book, name, new SizeT(name.length), rows));
  }

  /** The next row of {@code rows}, its values lent until the next call on it. */
  Next rowsNext(Pointer rows) {
    IntByReference number = new IntByReference();
    PointerByReference values = new PointerByReference();
    SizeTByReference count = new SizeTByReference();
    int status = noted(api.xlsx_graft_rows_next(rows, number, values, count));
    return status == StatusDiagnostic.OK
        ? new Next(status, number.getValue() & 0xFFFFFFFFL, values.getValue(), count.getValue())
        : new Next(status, 0, null, 0);
  }

  int rowsClose(Pointer rows) {
    return noted(api.xlsx_graft_rows_close(rows));
  }

  /**
   * Why the last call through this Capi that returned {@code XLSX_GRAFT_ERR_IO} failed, read as it
   * returned; empty when the library said nothing.
   */
  String reason() {
    return reason;
  }

  // Keeps xlsx_graft_error_message's reason for an IO `status`, read now: it
  // is the OS thread's, and the next failure on the thread replaces it.
  private int noted(int status) {
    if (status == StatusDiagnostic.IO) {
      int read = errorMessage.invokeInt(new Object[] {lent, lentLength});
      reason = read == StatusDiagnostic.OK ? text(lent.getPointer(0), sizeT(lentLength, 0)) : "";
    }
    return status;
  }

  /** A {@code size_t} at {@code offset} of {@code memory}. */
  static long sizeT(Pointer memory, long offset) {
    return Native.SIZE_T_SIZE == 8 ? memory.getLong(offset) : memory.getInt(offset) & 0xFFFFFFFFL;
  }

  /** What {@code xlsx_graft_rows_next} wrote. */
  static final class Next {

    final int status;

    /** The row's number, 0 at the end. */
    final long number;

    /** The XlsxGraftValue array, null for an empty row or a status other than OK. */
    final Pointer values;

    final long count;

    Next(int status, long number, Pointer values, long count) {
      this.status = status;
      this.number = number;
      this.values = values;
      this.count = count;
    }
  }

  /** What {@code xlsx_graft_book_diagnostics} wrote, copied out. */
  static final class Lent {

    final int status;

    /** The JSON's bytes, empty for a status other than OK. */
    final byte[] bytes;

    Lent(int status, byte[] bytes) {
      this.status = status;
      this.bytes = bytes;
    }
  }
}
