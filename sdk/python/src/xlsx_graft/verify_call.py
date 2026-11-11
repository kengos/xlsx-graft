"""The C ABI's verify functions (engine/capi/include/xlsx_graft.h, section
VERIFY). Each method makes the verify handle, copies the result's JSON out
and closes the handle before it returns, so no handle outlives the call
(docs/sdk.md, "Verifying through the C ABI"). Every input is ``bytes``,
which no thread can change while the engine reads it (ctypes releases the
GIL during a call).

A :class:`VerifyCall` holds only the library's functions and does not change
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
    "verify_path": (ctypes.c_char_p, SIZE, ctypes.c_char_p, SIZE, OUT_HANDLE),
    "verify_bytes": (ctypes.c_char_p, SIZE, ctypes.c_char_p, SIZE, OUT_HANDLE),
    "policy_check": (ctypes.c_char_p, SIZE, OUT_HANDLE),
    "verify_result": (HANDLE, OUT_BYTES, OUT_SIZE),
    "verify_close": (HANDLE,),
}


class VerifyCall:
    """The verify functions of one engine library. Every method returns the
    status first, then the result's JSON as the bytes the engine wrote, not
    decoded, which are empty unless the status is ``XLSX_GRAFT_OK``.
    """

    def __init__(self, library: Library) -> None:
        """Raises :class:`~xlsx_graft.LibraryNotFoundError` when the library
        lacks one of the functions.
        """
        self._functions = {
            name: library.function(f"xlsx_graft_{name}", argtypes, ctypes.c_int32)
            for name, argtypes in SIGNATURES.items()
        }

    def verify_path(self, path: bytes, options: bytes) -> tuple[int, bytes]:
        """Verifies the workbook at ``path`` (UTF-8) under ``options``, the
        JSON ``{"policy": ..., "rules": ...}``.
        """
        return self._made("verify_path", path, len(path), options, len(options))

    def verify_bytes(self, data: bytes, options: bytes) -> tuple[int, bytes]:
        """Verifies the workbook in ``data`` under ``options``."""
        return self._made("verify_bytes", data, len(data), options, len(options))

    def check_policy(self, policy: bytes) -> tuple[int, bytes]:
        """Checks ``policy``, the policy's JSON object itself, not wrapped in
        ``{"policy": ...}``.
        """
        return self._made("policy_check", policy, len(policy))

    # Calls ``name``, copies out the result of the handle it wrote and closes
    # the handle. ``out`` is made before the call and the handle is closed in
    # ``finally``, so a handle the library wrote is closed even when the call
    # to verify_result raises (a KeyboardInterrupt, or an exception of the
    # function called).
    def _made(self, name: str, *args: Any) -> tuple[int, bytes]:
        out = HANDLE()
        try:
            status: int = self._functions[name](*args, ctypes.byref(out))
            if status != status_diagnostic.OK:
                return status, b""
            return self._lent(out)
        finally:
            if out.value is not None:
                self._functions["verify_close"](out)

    # The status of verify_result, and the handle's JSON copied out as bytes.
    def _lent(self, handle: HANDLE) -> tuple[int, bytes]:
        address = BYTES()
        length = SIZE()
        status: int = self._functions["verify_result"](
            handle, ctypes.byref(address), ctypes.byref(length)
        )
        if status != status_diagnostic.OK:
            return status, b""
        # An empty result may come as NULL, which string_at does not take.
        return status, (ctypes.string_at(address, length.value) if length.value else b"")
