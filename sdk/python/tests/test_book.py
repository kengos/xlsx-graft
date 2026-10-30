"""What the engine cannot be made to do from a workbook: panic, fail the host
while lending the diagnostics, answer a status or lend a value newer than the
package. Each test reads a real workbook through a Capi of which one call is
replaced.
"""

from __future__ import annotations

import ctypes
import gc
from typing import Any

import pytest

import workbook
from xlsx_graft import XlsxReader, cell_values
from xlsx_graft.capi import Capi, CValue
from xlsx_graft.result import Result

BYTES = workbook.xlsx(workbook.worksheet("Data", '<row r="1"/><row r="2"/>'))


def read() -> tuple[Result[None], list[int]]:
    """Reads the first sheet: the result and the rows' numbers."""
    with XlsxReader.open_bytes(BYTES) as book:
        rows = [row.number for row in book.sheet().rows()]
    return book.result, rows


def replace(monkeypatch: pytest.MonkeyPatch, name: str, *returned: Any) -> list[Any]:
    """Makes Capi's ``name`` return ``returned`` (one value, or a tuple), and
    the calls it gets.
    """
    calls: list[Any] = []

    def fake(_capi: Capi, *args: Any) -> Any:
        calls.append(args)
        return returned[0] if len(returned) == 1 else returned

    monkeypatch.setattr(Capi, name, fake)
    return calls


def spy(monkeypatch: pytest.MonkeyPatch, name: str) -> list[Any]:
    """Counts the calls to Capi's ``name``, which goes on to the library."""
    calls: list[Any] = []
    original = getattr(Capi, name)

    def counted(capi: Capi, *args: Any) -> Any:
        calls.append(args)
        return original(capi, *args)

    monkeypatch.setattr(Capi, name, counted)
    return calls


def test_a_panic_reading_a_row_fails_with_panic_reading_no_diagnostics(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # The engine refuses every call after a panic.
    replace(monkeypatch, "rows_next", 2, 0, None, 0)
    replace(monkeypatch, "book_diagnostics", 2, "")
    result, rows = read()
    assert (result.success, result.diagnostics, rows) == (False, (), [])
    assert result.error is not None
    assert result.error.code == "panic"


def test_a_status_the_package_does_not_know_fails_with_unknown_status(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    replace(monkeypatch, "rows_open", 42)
    result, _ = read()
    assert result.error is not None
    assert result.error.code == "unknown_status"
    assert "42" in result.error.message


def test_a_value_of_a_kind_newer_than_the_package_stops_the_sheet_with_unknown_value(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    replace(monkeypatch, "rows_next", 0, 1, ctypes.pointer(CValue(kind=9)), 1)
    result, rows = read()
    assert rows == []
    assert result.error is not None
    assert result.error.code == "unknown_value"
    assert result.error.message == (
        "row 1: the engine returned a value of kind 9, which this package does not know"
    )


def test_a_stop_of_the_packages_before_a_refused_sheet_is_kept_as_the_error(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def refused(*_args: Any) -> Any:
        raise cell_values.RefusedError("unknown_value", "kind 9")

    monkeypatch.setattr(cell_values, "call", refused)
    with XlsxReader.open_bytes(BYTES) as book:
        list(book.sheet().rows())
        list(book.sheet("Missing").rows())
    assert book.result.error is not None
    assert book.result.error.code == "unknown_value"


def test_diagnostics_that_cannot_be_lent_fail_with_io_saying_why(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    replace(monkeypatch, "book_diagnostics", 3, "")
    replace(monkeypatch, "error_message", "the disk is full")
    result, rows = read()
    assert rows == [1, 2]
    assert result.error is not None
    assert (result.error.code, result.error.message) == ("io", "the disk is full")


@pytest.mark.parametrize("json", ["{}", "[]", "not json", '{"error": {}}'])
def test_diagnostics_not_of_the_shape_fail_with_diagnostics_unreadable(
    monkeypatch: pytest.MonkeyPatch, json: str
) -> None:
    replace(monkeypatch, "book_diagnostics", 0, json)
    result, _ = read()
    assert result.error is not None
    assert result.error.code == "diagnostics_unreadable"
    assert result.error.message.startswith("the engine's diagnostics could not be read: ")


def test_a_book_the_caller_closes_in_the_block_is_closed_once(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    closes = spy(monkeypatch, "book_close")
    with XlsxReader.open_bytes(BYTES) as book:
        book.close()
        book.close()
    assert book.result.success
    assert len(closes) == 1


def test_the_rows_a_generator_left_open_are_freed_once_at_the_end(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    closes = spy(monkeypatch, "rows_close")
    with XlsxReader.open_bytes(BYTES) as book:
        rows = book.sheet().rows()
        next(rows)
    rows.close()
    assert len(closes) == 1


def test_a_panic_freeing_a_rows_handle_fails_with_panic(monkeypatch: pytest.MonkeyPatch) -> None:
    replace(monkeypatch, "rows_close", 2)
    result, rows = read()
    assert rows == [1, 2]
    assert result.error is not None
    assert result.error.code == "panic"


def test_a_sheet_of_a_kind_or_state_newer_than_the_package_is_listed_as_unknown(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    replace(monkeypatch, "book_sheets", 0, [("New", 7, 9)])
    with XlsxReader.open_bytes(BYTES) as book:
        (sheet,) = book.sheets()
    assert (sheet.kind, sheet.state) == ("unknown", "unknown")


def test_an_interrupt_after_the_open_closes_the_handle_it_made(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    closes = spy(monkeypatch, "book_close")
    handles: list[int | None] = []
    original = Capi.book_open_bytes

    def interrupted(capi: Capi, data: bytes, out: ctypes.c_void_p) -> int:
        original(capi, data, out)
        handles.append(out.value)
        raise KeyboardInterrupt

    monkeypatch.setattr(Capi, "book_open_bytes", interrupted)
    book = XlsxReader.open_bytes(BYTES)
    with pytest.raises(KeyboardInterrupt), book:
        pass
    assert handles[0] is not None
    assert len(closes) == 1
    assert book.result.success


def raising(monkeypatch: pytest.MonkeyPatch, name: str, error: BaseException) -> None:
    def failing(*_args: Any) -> Any:
        raise error

    monkeypatch.setattr(Capi, name, failing)


@pytest.mark.parametrize("name", ["book_diagnostics", "book_close"])
def test_a_close_an_interrupt_cuts_short_fails_with_interrupted(
    monkeypatch: pytest.MonkeyPatch, name: str
) -> None:
    closes = spy(monkeypatch, "book_close")
    raising(monkeypatch, name, KeyboardInterrupt())
    book = XlsxReader.open_bytes(BYTES)
    with pytest.raises(KeyboardInterrupt), book:
        next(book.sheet().rows())
    assert len(closes) == (name == "book_diagnostics")
    assert (book.result.success, book.result.diagnostics) == (False, ())
    assert book.result.error is not None
    assert book.result.error.code == "interrupted"


def test_diagnostics_that_are_not_utf_8_fail_with_diagnostics_unreadable(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    raising(monkeypatch, "book_diagnostics", UnicodeDecodeError("utf-8", b"\xff", 0, 1, "bad"))
    result, _ = read()
    assert result.error is not None
    assert result.error.code == "diagnostics_unreadable"


def test_a_diagnostic_hashes_by_its_code_severity_and_message() -> None:
    with XlsxReader.open_bytes(workbook.xlsx(workbook.unrelated("Lost"))) as book:
        pass
    assert len(set(book.result.diagnostics)) == len(book.result.diagnostics) > 0
    assert hash(book.result)


def test_an_interrupt_after_a_rows_open_leaves_the_handle_for_the_book_to_free(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    closes = spy(monkeypatch, "rows_close")
    handles: list[int | None] = []
    original = Capi.rows_open

    def interrupted(capi: Capi, book: ctypes.c_void_p, index: int, out: ctypes.c_void_p) -> int:
        original(capi, book, index, out)
        handles.append(out.value)
        raise KeyboardInterrupt

    monkeypatch.setattr(Capi, "rows_open", interrupted)
    with pytest.raises(KeyboardInterrupt), XlsxReader.open_bytes(BYTES) as book:
        next(book.sheet().rows())
    assert handles[0] is not None
    assert [args[0].value for args in closes] == [handles[0]]


def test_a_generator_collected_mid_sheet_frees_its_rows_once(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    closes = spy(monkeypatch, "rows_close")
    with XlsxReader.open_bytes(BYTES) as book:
        rows = book.sheet().rows()
        next(rows)
        del rows
        gc.collect()
        assert len(closes) == 1
    assert len(closes) == 1
    assert book.result.success


def test_a_generator_collected_during_a_read_frees_its_rows_under_the_same_lock(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    closes = spy(monkeypatch, "rows_close")
    original = cell_values.call

    def collecting(values: Any, count: int) -> Any:
        gc.collect()
        return original(values, count)

    # Off, so that only the read's own collection frees the cycle.
    gc.disable()
    try:
        with XlsxReader.open_bytes(BYTES) as book:
            # Garbage only the cycle collector frees, holding a generator
            # midway.
            cycle: list[Any] = [book.sheet().rows()]
            cycle.append(cycle)
            next(cycle[0])
            del cycle
            monkeypatch.setattr(cell_values, "call", collecting)
            numbers = [row.number for row in book.sheet().rows()]
    finally:
        gc.enable()
    assert numbers == [1, 2]
    assert len(closes) == 2
    assert book.result.success
