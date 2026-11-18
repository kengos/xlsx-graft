from __future__ import annotations

import ctypes
from pathlib import Path
from typing import Any

import pytest

from xlsx_graft.library import Library
from xlsx_graft.view_call import ViewCall

MERGED = Path(__file__).resolve().parents[3] / "fixtures" / "merged.xlsx"
OK = b'{"ok":true'


@pytest.fixture
def call(engine_library: str) -> ViewCall:
    return ViewCall(Library.open(env={"XLSX_GRAFT_LIBRARY": engine_library}))


def spy_on_close(call: ViewCall) -> list[object]:
    closed: list[object] = []
    real = call._functions["view_close"]

    def close(handle: Any) -> int:
        closed.append(handle.value)
        status: int = real(handle)
        return status

    call._functions["view_close"] = close
    return closed


def lend_bytes(data: bytes) -> Any:
    """A function that writes the address and the length of ``data``."""
    buffer = (ctypes.c_uint8 * len(data))(*data)

    def lend(*args: Any) -> int:
        address, length = args[-2:]
        slot = ctypes.cast(ctypes.pointer(address._obj), ctypes.POINTER(ctypes.c_void_p))
        slot[0] = ctypes.addressof(buffer)
        length._obj.value = len(data)
        return 0

    return lend


def test_returns_the_status_and_the_views_json_for_a_path_and_bytes_of_each_view(
    call: ViewCall,
) -> None:
    path = str(MERGED).encode()
    data = MERGED.read_bytes()
    status, json = call.call("describe_sheet_path", path, b"")
    assert status == 0
    assert json.startswith(OK)
    assert call.call("describe_sheet_bytes", data, b"") == (0, json)
    for name, input_ in (("tree_bytes", data), ("tree_path", path)):
        status, tree = call.call(name, input_, b"")
        assert (status, tree.startswith(OK)) == (0, True)


def test_returns_the_engines_message_with_request_invalid_for_options_it_refuses(
    call: ViewCall,
) -> None:
    closed = spy_on_close(call)
    status, message = call.call("describe_sheet_path", str(MERGED).encode(), b'{"maxRows":0}')
    assert status == 11
    assert b"maxRows 0" in message
    assert closed == []


def test_returns_no_bytes_with_request_invalid_when_the_message_cannot_be_read(
    call: ViewCall,
) -> None:
    call._functions["error_message"] = lambda *_args: 2
    options = b'{"maxRows":0}'
    assert call.call("describe_sheet_path", str(MERGED).encode(), options) == (11, b"")


def test_returns_invalid_arg_and_no_bytes_for_options_that_are_not_an_object(
    call: ViewCall,
) -> None:
    assert call.call("describe_sheet_path", str(MERGED).encode(), b"[]") == (4, b"")


def test_closes_the_handle_after_copying_the_result_out(call: ViewCall) -> None:
    closed = spy_on_close(call)
    assert call.call("tree_bytes", b"", b"")[0] == 0
    assert len(closed) == 1
    assert closed[0] is not None


def test_closes_the_handle_when_the_result_cannot_be_lent(call: ViewCall) -> None:
    closed = spy_on_close(call)
    call._functions["view_result"] = lambda *_args: 2
    assert call.call("tree_bytes", b"", b"") == (2, b"")
    assert len(closed) == 1


def test_closes_the_handle_when_copying_the_result_out_raises(call: ViewCall) -> None:
    closed = spy_on_close(call)

    def raises(*_args: object) -> int:
        raise RuntimeError("boom")

    call._functions["view_result"] = raises
    with pytest.raises(RuntimeError, match="boom"):
        call.call("tree_bytes", b"", b"")
    assert len(closed) == 1


def test_returns_the_results_bytes_without_decoding_them(call: ViewCall) -> None:
    call._functions["view_result"] = lend_bytes(b"\xff\xfe")
    assert call.call("tree_bytes", b"", b"") == (0, b"\xff\xfe")


def test_returns_no_bytes_for_an_empty_result(call: ViewCall) -> None:
    call._functions["view_result"] = lambda *_args: 0
    assert call.call("tree_bytes", b"", b"") == (0, b"")


def test_returns_the_message_as_the_bytes_the_engine_wrote(call: ViewCall) -> None:
    call._functions["describe_sheet_bytes"] = lambda *_args: 11
    call._functions["error_message"] = lend_bytes(b"\xff why")
    assert call.call("describe_sheet_bytes", b"", b"") == (11, b"\xff why")
