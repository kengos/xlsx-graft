from __future__ import annotations

import ctypes
from pathlib import Path
from typing import Any

import pytest

from xlsx_graft.library import Library
from xlsx_graft.verify_call import VerifyCall

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"
OPTIONS = b'{"policy":null,"rules":{}}'
CLEAN = (
    b'{"ok":true,"failOn":"error","counts":{"error":0,"warn":0,"notice":0},'
    b'"error":null,"diagnostics":[]}'
)


@pytest.fixture
def call(engine_library: str) -> VerifyCall:
    return VerifyCall(Library.open(env={"XLSX_GRAFT_LIBRARY": engine_library}))


def spy_on_close(call: VerifyCall) -> list[object]:
    closed: list[object] = []
    real = call._functions["verify_close"]

    def close(handle: Any) -> int:
        closed.append(handle.value)
        status: int = real(handle)
        return status

    call._functions["verify_close"] = close
    return closed


def test_returns_the_status_and_the_results_json_for_a_path_bytes_and_a_policy(
    call: VerifyCall,
) -> None:
    table = FIXTURES / "table.xlsx"
    assert call.verify_path(str(table).encode(), OPTIONS) == (0, CLEAN)
    assert call.verify_bytes(table.read_bytes(), OPTIONS) == (0, CLEAN)
    assert call.check_policy(b"{}") == (0, CLEAN)


def test_closes_the_handle_after_copying_the_result_out(call: VerifyCall) -> None:
    closed = spy_on_close(call)
    assert call.verify_bytes(b"", OPTIONS)[0] == 0
    assert len(closed) == 1
    assert closed[0] is not None


def test_returns_invalid_arg_and_no_json_for_options_that_are_not_an_object_of_the_keys(
    call: VerifyCall,
) -> None:
    closed = spy_on_close(call)
    assert call.verify_bytes(b"", b"[]") == (4, b"")
    assert closed == []


def test_closes_the_handle_when_the_result_cannot_be_lent(call: VerifyCall) -> None:
    closed = spy_on_close(call)
    call._functions["verify_result"] = lambda *_args: 2
    assert call.check_policy(b"{}") == (2, b"")
    assert len(closed) == 1


def test_closes_the_handle_when_copying_the_result_out_raises(call: VerifyCall) -> None:
    closed = spy_on_close(call)

    def raises(*_args: object) -> int:
        raise RuntimeError("boom")

    call._functions["verify_result"] = raises
    with pytest.raises(RuntimeError, match="boom"):
        call.check_policy(b"{}")
    assert len(closed) == 1


def lend_bytes(data: bytes) -> Any:
    """A verify_result that writes the address and the length of ``data``."""
    buffer = (ctypes.c_uint8 * len(data))(*data)

    def lend(_handle: object, address: Any, length: Any) -> int:
        slot = ctypes.cast(ctypes.pointer(address._obj), ctypes.POINTER(ctypes.c_void_p))
        slot[0] = ctypes.addressof(buffer)
        length._obj.value = len(data)
        return 0

    return lend


def test_returns_the_results_bytes_without_decoding_them(call: VerifyCall) -> None:
    call._functions["verify_result"] = lend_bytes(b"\xff\xfe")
    assert call.check_policy(b"{}") == (0, b"\xff\xfe")


def test_returns_no_bytes_for_an_empty_result(call: VerifyCall) -> None:
    call._functions["verify_result"] = lambda *_args: 0
    assert call.check_policy(b"{}") == (0, b"")
