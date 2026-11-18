"""The C ABI's view functions (engine/capi/include/xlsx_graft.h, section
VIEWS). Each call makes the view handle, copies the result's JSON out and
closes the handle before it returns, so no handle outlives the call
(docs/sdk.md, "Viewing through the C ABI"). Every input is ``bytes``, which
no thread can change while the engine reads it (ctypes releases the GIL
during a call).

A :class:`ViewCall` holds only the library's functions and does not change
after it is made, so any thread may use one at once.
"""

from __future__ import annotations

import ctypes
from typing import TYPE_CHECKING, Any

from xlsx_graft import status_diagnostic
from xlsx_graft.capi import BYTES, HANDLE, OUT_BYTES, OUT_HANDLE, OUT_SIZE, SIZE

if TYPE_CHECKING:
    from xlsx_graft.library import Library

# c_char_p for an input, as in xlsx_graft.capi.SIGNATURES.
SIGNATURES: dict[str, tuple[Any, ...]] = {
    "tree_path": (ctypes.c_char_p, SIZE, ctypes.c_char_p, SIZE, OUT_HANDLE),
    "tree_bytes": (ctypes.c_char_p, SIZE, ctypes.c_char_p, SIZE, OUT_HANDLE),
    "describe_sheet_path": (ctypes.c_char_p, SIZE, ctypes.c_char_p, SIZE, OUT_HANDLE),
    "describe_sheet_bytes": (ctypes.c_char_p, SIZE, ctypes.c_char_p, SIZE, OUT_HANDLE),
    "view_result": (HANDLE, OUT_BYTES, OUT_SIZE),
    "view_close": (HANDLE,),
    "error_message": (OUT_BYTES, OUT_SIZE),
}


class ViewCall:
    """The view functions of one engine library."""

    def __init__(self, library: Library) -> None:
        """Raises :class:`~xlsx_graft.LibraryNotFoundError` when the library
        lacks one of the functions.
        """
        self._functions = {
            name: library.function(f"xlsx_graft_{name}", argtypes, ctypes.c_int32)
            for name, argtypes in SIGNATURES.items()
        }

    def call(self, name: str, data: bytes, options: bytes) -> tuple[int, bytes]:
        """Calls ``name`` (``tree_path``, ``tree_bytes``,
        ``describe_sheet_path`` or ``describe_sheet_bytes``): ``data`` is the
        path as UTF-8 for a ``_path`` name and the workbook's bytes for a
        ``_bytes`` name, and ``options`` is the options' JSON (empty is
        every default).

        Returns the status first. The bytes after it are the view's JSON as
        the engine wrote it, not decoded, for ``XLSX_GRAFT_OK``; the
        message of ``XLSX_GRAFT_ERR_REQUEST_INVALID`` (read on this thread
        right after the call, as the header says; empty when it cannot be
        read); and empty for any other status.
        """
        return self._made(name, data, len(data), options, len(options))

    # Calls ``name``, copies out the result of the handle it wrote and closes
    # the handle. ``out`` is made before the call and the handle is closed in
    # ``finally``, so a handle the library wrote is closed even when the call
    # to view_result raises (a KeyboardInterrupt, or an exception of the
    # function called).
    def _made(self, name: str, *args: Any) -> tuple[int, bytes]:
        out = HANDLE()
        try:
            status: int = self._functions[name](*args, ctypes.byref(out))
            if status == status_diagnostic.OK:
                return self._copied("view_result", out)
            if status == status_diagnostic.REQUEST_INVALID:
                return status, self._copied("error_message")[1]
            return status, b""
        finally:
            if out.value is not None:
                self._functions["view_close"](out)

    # Calls ``name`` (view_result of a handle, or error_message) with the two
    # slots it writes the address and the length of some bytes to, and copies
    # those bytes out. The bytes are empty unless the status is OK; for
    # error_message they are the message of the call that returned
    # XLSX_GRAFT_ERR_REQUEST_INVALID.
    def _copied(self, name: str, *args: Any) -> tuple[int, bytes]:
        address = BYTES()
        length = SIZE()
        status: int = self._functions[name](*args, ctypes.byref(address), ctypes.byref(length))
        if status != status_diagnostic.OK:
            return status, b""
        # An empty result may come as NULL, which string_at does not take.
        return status, (ctypes.string_at(address, length.value) if length.value else b"")
