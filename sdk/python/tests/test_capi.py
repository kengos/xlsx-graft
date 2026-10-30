from __future__ import annotations

import ctypes

from xlsx_graft.capi import Capi, CSheet, CValue
from xlsx_graft.library import Library


# The engine is built for 64-bit targets only, whose layout the header gives.
def test_the_structs_take_the_headers_layout() -> None:
    assert (ctypes.sizeof(CValue), CValue.number.offset, CValue.text_len.offset) == (48, 16, 40)
    assert (ctypes.sizeof(CSheet), CSheet.kind.offset, CSheet.state.offset) == (24, 16, 20)


def test_a_null_handle_is_refused_and_nothing_written_is_read(engine_library: str) -> None:
    capi = Capi(Library.open(path=engine_library))
    null = ctypes.c_void_p()
    assert capi.rows_next(null) == (1, 0, None, 0)
    assert capi.book_sheets(null) == (1, [])
    assert capi.book_diagnostics(null) == (1, "")
    assert capi.rows_close(null) == 0
