// The C ABI, written from engine/capi/include/xlsx_graft.h:
//
//     int32_t xlsx_graft_version(const uint8_t **out_ptr, size_t *out_len);
//
// Interface mapping rather than direct mapping (Native.register): direct
// mapping binds one library to a class when the class is initialized, once
// per process, so it cannot follow docs/sdk.md's lookup order per call.
//
// No String crosses the boundary: JNA reads a String as a NUL-terminated C
// string in the platform's charset, and the engine's strings are neither.

package io.github.kengos.xlsxgraft;

import com.sun.jna.ptr.PointerByReference;

/** The engine's functions, as JNA binds them. */
interface XlsxGraftLibrary extends com.sun.jna.Library {

  /** The engine version's UTF-8 bytes and their count. */
  int xlsx_graft_version(PointerByReference outPtr, SizeTByReference outLen);
}
