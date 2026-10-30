"""The C ABI's book and rows functions (engine/capi/include/xlsx_graft.h),
declared with their types. Each method makes one call and returns its status
first, then what it wrote; what it wrote is only meaningful when the status
is ``XLSX_GRAFT_OK``.

A handle is a ``c_void_p`` the caller owns and the library writes into, so a
handle the library made is kept even when an exception comes between the
call and the next line; NULL is ``None``. What the library lends (sheet
names, the diagnostics' JSON) is copied out before the method returns. A
row's values are not: :mod:`xlsx_graft.cell_values` reads them, before the
next call on that rows handle, or its close.

Every input is ``bytes``, which no thread can change while the engine reads
it (ctypes lets go of the GIL during a call).

Thread safety is the caller's: nothing here locks.
"""

from __future__ import annotations

import ctypes
from typing import TYPE_CHECKING, Any

from xlsx_graft import status_diagnostic

if TYPE_CHECKING:
    from xlsx_graft.library import Library

BYTES = ctypes.POINTER(ctypes.c_uint8)
HANDLE = ctypes.c_void_p
SIZE = ctypes.c_size_t


class CValue(ctypes.Structure):
    """``XlsxGraftValue``: 48 bytes on a 64-bit target."""

    _fields_ = (
        ("kind", ctypes.c_uint32),
        ("year", ctypes.c_uint16),
        ("month", ctypes.c_uint8),
        ("day", ctypes.c_uint8),
        ("hour", ctypes.c_uint8),
        ("minute", ctypes.c_uint8),
        ("second", ctypes.c_uint8),
        ("boolean", ctypes.c_uint8),
        ("millisecond", ctypes.c_uint16),
        ("reserved", ctypes.c_uint16),
        ("number", ctypes.c_double),
        ("elapsed", ctypes.c_uint64),
        ("text", BYTES),
        ("text_len", SIZE),
    )


class CSheet(ctypes.Structure):
    """``XlsxGraftSheet``: 24 bytes on a 64-bit target."""

    _fields_ = (
        ("name", BYTES),
        ("name_len", SIZE),
        ("kind", ctypes.c_uint32),
        ("state", ctypes.c_uint32),
    )


OUT_BYTES = ctypes.POINTER(BYTES)
OUT_HANDLE = ctypes.POINTER(HANDLE)
OUT_SIZE = ctypes.POINTER(SIZE)

# c_char_p, not a pointer to bytes, for an input: ctypes passes a bytes
# object's own buffer, which the length bounds (it is never read as a C
# string).
SIGNATURES: dict[str, tuple[Any, ...]] = {
    "book_open_path": (ctypes.c_char_p, SIZE, OUT_HANDLE),
    "book_open_bytes": (ctypes.c_char_p, SIZE, OUT_HANDLE),
    "book_sheets": (HANDLE, ctypes.POINTER(ctypes.POINTER(CSheet)), OUT_SIZE),
    "book_diagnostics": (HANDLE, OUT_BYTES, OUT_SIZE),
    "book_close": (HANDLE,),
    "rows_open": (HANDLE, SIZE, OUT_HANDLE),
    "rows_open_name": (HANDLE, ctypes.c_char_p, SIZE, OUT_HANDLE),
    "rows_next": (
        HANDLE,
        ctypes.POINTER(ctypes.c_uint32),
        ctypes.POINTER(ctypes.POINTER(CValue)),
        OUT_SIZE,
    ),
    "rows_close": (HANDLE,),
    "error_message": (OUT_BYTES, OUT_SIZE),
}


def text(address: Any, length: int) -> str:
    """The ``length`` UTF-8 bytes at ``address``, copied out, never read as
    a C string. An empty text may come as NULL.
    """
    if length == 0:
        return ""
    return ctypes.string_at(address, length).decode("utf-8")


class Capi:
    """The functions of one engine library."""

    def __init__(self, library: Library) -> None:
        """Raises :class:`~xlsx_graft.LibraryNotFoundError` when the library
        lacks one of the functions.
        """
        self._functions = {
            name: library.function(f"xlsx_graft_{name}", argtypes, ctypes.c_int32)
            for name, argtypes in SIGNATURES.items()
        }

    def book_open_path(self, path: bytes, out: HANDLE) -> int:
        """Writes the book handle into ``out``."""
        return self._call("book_open_path", path, len(path), ctypes.byref(out))

    def book_open_bytes(self, data: bytes, out: HANDLE) -> int:
        """Writes the book handle into ``out``."""
        return self._call("book_open_bytes", data, len(data), ctypes.byref(out))

    def book_sheets(self, book: HANDLE) -> tuple[int, list[tuple[str, int, int]]]:
        """The status and each sheet's name, kind and state."""
        sheets = ctypes.POINTER(CSheet)()
        count = SIZE()
        status = self._call("book_sheets", book, ctypes.byref(sheets), ctypes.byref(count))
        if status != status_diagnostic.OK:
            return status, []
        listed = [sheets[index] for index in range(count.value)]
        return status, [(text(s.name, s.name_len), s.kind, s.state) for s in listed]

    def book_diagnostics(self, book: HANDLE) -> tuple[int, str]:
        """The status and the diagnostics' JSON."""
        return self._lent("book_diagnostics", book)

    def book_close(self, book: HANDLE) -> int:
        return self._call("book_close", book)

    def rows_open(self, book: HANDLE, index: int, out: HANDLE) -> int:
        """Writes the rows handle into ``out``. ``index`` is from 0 to what a
        size_t holds.
        """
        return self._call("rows_open", book, index, ctypes.byref(out))

    def rows_open_name(self, book: HANDLE, name: bytes, out: HANDLE) -> int:
        """Writes the rows handle into ``out``."""
        return self._call("rows_open_name", book, name, len(name), ctypes.byref(out))

    def rows_next(self, rows: HANDLE) -> tuple[int, int, Any, int]:
        """The status, the row's number (0 at the end), and its values and
        their count.
        """
        number = ctypes.c_uint32()
        values = ctypes.POINTER(CValue)()
        count = SIZE()
        status = self._call(
            "rows_next", rows, ctypes.byref(number), ctypes.byref(values), ctypes.byref(count)
        )
        if status != status_diagnostic.OK:
            return status, 0, None, 0
        return status, number.value, values, count.value

    def rows_close(self, rows: HANDLE) -> int:
        return self._call("rows_close", rows)

    def error_message(self) -> str:
        """Why the last call on this thread that returned
        ``XLSX_GRAFT_ERR_IO`` failed: the operating system's thread, so call
        it right after the call that failed.
        """
        _status, message = self._lent("error_message")
        return message

    def _call(self, name: str, *args: Any) -> int:
        status: int = self._functions[name](*args)
        return status

    # Calls a function whose last two parameters are a pointer and a length
    # it writes.
    def _lent(self, name: str, *args: Any) -> tuple[int, str]:
        address = BYTES()
        length = SIZE()
        status = self._call(name, *args, ctypes.byref(address), ctypes.byref(length))
        if status != status_diagnostic.OK:
            return status, ""
        return status, text(address, length.value)
