// The C ABI, written from engine/capi/include/xlsx_graft.h:
//
//     int32_t xlsx_graft_version(const uint8_t **out_ptr, size_t *out_len);
//     int32_t xlsx_graft_book_open_path(const uint8_t *path, size_t path_len,
//                                       XlsxGraftBook **out_book);
//     int32_t xlsx_graft_book_open_bytes(const uint8_t *data, size_t len,
//                                        XlsxGraftBook **out_book);
//     int32_t xlsx_graft_book_sheets(XlsxGraftBook *book,
//                                    const XlsxGraftSheet **out_sheets, size_t *out_len);
//     int32_t xlsx_graft_book_diagnostics(XlsxGraftBook *book,
//                                         const uint8_t **out_ptr, size_t *out_len);
//     int32_t xlsx_graft_book_close(XlsxGraftBook *book);
//     int32_t xlsx_graft_rows_open(XlsxGraftBook *book, size_t index,
//                                  XlsxGraftRows **out_rows);
//     int32_t xlsx_graft_rows_open_name(XlsxGraftBook *book, const uint8_t *name,
//                                       size_t name_len, XlsxGraftRows **out_rows);
//     int32_t xlsx_graft_rows_next(XlsxGraftRows *rows, uint32_t *out_number,
//                                  const XlsxGraftValue **out_values, size_t *out_len);
//     int32_t xlsx_graft_rows_close(XlsxGraftRows *rows);
//     int32_t xlsx_graft_verify_path(const uint8_t *path, size_t path_len,
//                                    const uint8_t *options, size_t options_len,
//                                    XlsxGraftVerify **out_verify);
//     int32_t xlsx_graft_verify_bytes(const uint8_t *data, size_t len,
//                                     const uint8_t *options, size_t options_len,
//                                     XlsxGraftVerify **out_verify);
//     int32_t xlsx_graft_policy_check(const uint8_t *policy, size_t len,
//                                     XlsxGraftVerify **out_verify);
//     int32_t xlsx_graft_verify_result(const XlsxGraftVerify *verify,
//                                      const uint8_t **out_ptr, size_t *out_len);
//     int32_t xlsx_graft_verify_close(XlsxGraftVerify *verify);
//
// Interface mapping rather than direct mapping (Native.register): direct
// mapping binds one library to a class when the class is initialized, once
// per process, so it cannot follow docs/sdk.md's lookup order per call.
//
// No String crosses the boundary: JNA reads a String as a NUL-terminated C
// string in the platform's charset, and the engine's strings are neither. A
// size_t is a SizeT, never a long, which is 64 bits on every platform.
// xlsx_graft_error_message is called as a Function, not through this
// interface: Capi says why.

package io.github.kengos.xlsxgraft;

import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.PointerByReference;

/** The engine's functions, as JNA binds them. */
interface XlsxGraftLibrary extends com.sun.jna.Library {

  /** The engine version's UTF-8 bytes and their count. */
  int xlsx_graft_version(PointerByReference outPtr, SizeTByReference outLen);

  /** Opens the workbook at the UTF-8 path, writing the book handle to {@code outBook}. */
  int xlsx_graft_book_open_path(byte[] path, SizeT pathLen, PointerByReference outBook);

  /** Opens the workbook of {@code data}, writing the book handle to {@code outBook}. */
  int xlsx_graft_book_open_bytes(byte[] data, SizeT len, PointerByReference outBook);

  /** The book's XlsxGraftSheet array and its count. */
  int xlsx_graft_book_sheets(Pointer book, PointerByReference outSheets, SizeTByReference outLen);

  /** The book's diagnostics, UTF-8 JSON, and the count of its bytes. */
  int xlsx_graft_book_diagnostics(Pointer book, PointerByReference outPtr, SizeTByReference outLen);

  /** Closes the book and the rows of every rows handle opened from it. */
  int xlsx_graft_book_close(Pointer book);

  /** Opens the rows of the sheet at {@code index}, writing the rows handle to {@code outRows}. */
  int xlsx_graft_rows_open(Pointer book, SizeT index, PointerByReference outRows);

  /** Opens the rows of the sheet of the UTF-8 name, writing the rows handle to {@code outRows}. */
  int xlsx_graft_rows_open_name(
      Pointer book, byte[] name, SizeT nameLen, PointerByReference outRows);

  /** The next row's number (0 at the end), its XlsxGraftValue array and its count. */
  int xlsx_graft_rows_next(
      Pointer rows,
      IntByReference outNumber,
      PointerByReference outValues,
      SizeTByReference outLen);

  /** Closes the rows handle. */
  int xlsx_graft_rows_close(Pointer rows);

  /** Verifies the workbook at the UTF-8 path under the UTF-8 JSON options, writing the handle. */
  int xlsx_graft_verify_path(
      byte[] path, SizeT pathLen, byte[] options, SizeT optionsLen, PointerByReference outVerify);

  /** Verifies the workbook of {@code data} under the UTF-8 JSON options, writing the handle. */
  int xlsx_graft_verify_bytes(
      byte[] data, SizeT len, byte[] options, SizeT optionsLen, PointerByReference outVerify);

  /** Checks the UTF-8 JSON policy alone, writing a handle whose result says whether it is valid. */
  int xlsx_graft_policy_check(byte[] policy, SizeT len, PointerByReference outVerify);

  /** The verify handle's result, UTF-8 JSON, and the count of its bytes. */
  int xlsx_graft_verify_result(Pointer verify, PointerByReference outPtr, SizeTByReference outLen);

  /** Closes the verify handle. */
  int xlsx_graft_verify_close(Pointer verify);
}
