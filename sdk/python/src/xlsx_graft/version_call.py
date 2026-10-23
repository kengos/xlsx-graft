"""Calls ``xlsx_graft_version`` and returns the engine's version as a
:class:`Result`.
"""

from __future__ import annotations

import ctypes
from typing import TYPE_CHECKING

from xlsx_graft import status_diagnostic
from xlsx_graft.result import Result

if TYPE_CHECKING:
    from xlsx_graft.library import Library

ARGTYPES = (ctypes.POINTER(ctypes.POINTER(ctypes.c_uint8)), ctypes.POINTER(ctypes.c_size_t))
RESTYPE = ctypes.c_int32


def call(library: Library) -> Result[str]:
    """The version string, such as ``"0.0.0"``, on success."""
    function = library.function("xlsx_graft_version", ARGTYPES, RESTYPE)
    out_ptr = ctypes.POINTER(ctypes.c_uint8)()
    out_len = ctypes.c_size_t()
    status = function(ctypes.byref(out_ptr), ctypes.byref(out_len))
    if status != status_diagnostic.OK:
        return Result.failure((status_diagnostic.call(status),))

    # The bytes are not NUL-terminated, so exactly `out_len` of them are
    # copied out.
    return Result.ok(ctypes.string_at(out_ptr, out_len.value).decode("utf-8"))
