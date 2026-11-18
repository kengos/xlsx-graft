from __future__ import annotations

from typing import Any

import pytest

from xlsx_graft import view_options


def options(
    sheet: Any = None, range_: Any = None, max_rows: Any = None, max_columns: Any = None
) -> bytes:
    return view_options.call(sheet, range_, max_rows, max_columns)


def test_writes_no_key_for_a_value_that_is_none() -> None:
    assert options() == b"{}"
    assert options(sheet="S") == b'{"sheet":"S"}'
    assert options(max_columns=3) == b'{"maxColumns":3}'


def test_writes_an_int_sheet_as_sheet_index_and_the_other_sheets_as_sheet() -> None:
    assert options(sheet=2) == b'{"sheetIndex":2}'
    assert options(sheet="Data") == b'{"sheet":"Data"}'
    assert options(sheet=1.5) == b'{"sheet":1.5}'


def test_writes_a_bool_sheet_as_sheet_for_the_engine_to_refuse() -> None:
    assert options(sheet=True) == b'{"sheet":true}'
    assert options(sheet=False) == b'{"sheet":false}'


def test_writes_every_key_in_the_order_sheet_range_max_rows_max_columns() -> None:
    result = options(sheet=1, range_="A1:B2", max_rows=5, max_columns=6)
    assert result == b'{"sheetIndex":1,"range":"A1:B2","maxRows":5,"maxColumns":6}'


def test_writes_text_outside_ascii_as_escapes() -> None:
    assert options(sheet="シ") == b'{"sheet":"\\u30b7"}'


@pytest.mark.parametrize(
    ("keyword", "arguments"),
    [
        ("max_rows", {"max_rows": 10**400}),
        ("sheet", {"sheet": -(10**400)}),
        ("range", {"range_": [10**400]}),
        ("range", {"range_": (10**400,)}),
        ("sheet", {"sheet": {"a": 10**400}}),
        ("max_columns", {"max_columns": 10**400}),
    ],
)
def test_raises_value_error_for_an_int_beyond_a_double_also_inside_a_list_or_a_dict(
    keyword: str, arguments: dict[str, Any]
) -> None:
    with pytest.raises(
        ValueError, match=rf"^{keyword}: an integer of 401 digits is beyond what the engine reads$"
    ):
        options(**arguments)


def test_takes_an_int_as_large_as_a_double_holds() -> None:
    assert options(max_rows=10**308).startswith(b'{"maxRows":1')


@pytest.mark.parametrize(
    "arguments",
    [
        {"max_columns": float("nan")},
        {"max_rows": float("inf")},
        {"range_": "\ud800"},
        {"range_": {"\ud800": 1}},
    ],
)
def test_raises_value_error_for_a_nan_an_infinity_or_a_lone_surrogate(
    arguments: dict[str, Any],
) -> None:
    with pytest.raises(ValueError):  # noqa: PT011 (the messages are json's and the package's)
        options(**arguments)


def test_raises_type_error_for_a_value_json_cannot_write() -> None:
    with pytest.raises(TypeError):
        options(range_=object())


def test_raises_value_error_for_a_cyclic_value_and_recursion_error_for_a_deep_one() -> None:
    cycle: list[Any] = []
    cycle.append(cycle)
    deep: list[Any] = []
    for _ in range(100_000):
        deep = [deep]
    with pytest.raises(ValueError, match="Circular reference"):
        options(range_=cycle)
    with pytest.raises(RecursionError):
        options(range_=deep)


def test_takes_options_of_exactly_2_mib_and_raises_value_error_for_one_byte_more() -> None:
    overhead = len(b'{"range":""}')
    name = "A" * ((2 << 20) - overhead)
    assert len(options(range_=name)) == 2 << 20
    with pytest.raises(
        ValueError, match=r"^the options are 2097153 bytes of JSON; the engine takes 2097152$"
    ):
        options(range_=name + "A")
