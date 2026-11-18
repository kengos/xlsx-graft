"""Writes the options of a ``describe_sheet`` call as the JSON the engine
takes (engine/capi/include/xlsx_graft.h, section VIEWS), and refuses the
values the engine could not read as JSON, so that a fault of the caller's
argument is an exception before the C ABI is called.
"""

from __future__ import annotations

import sys
from typing import Any

from xlsx_graft.policy import rules_json

# The most JSON the engine takes as options.
MAX_BYTES = 2 << 20


def call(sheet: Any, range_: Any, max_rows: Any, max_columns: Any) -> bytes:
    """The options' JSON as ASCII. ``sheet`` that is an ``int`` (a ``bool``
    is not one) is written as ``sheetIndex``, and any other ``sheet`` that
    is not ``None`` as ``sheet``; ``range_`` is ``range``, ``max_rows``
    ``maxRows`` and ``max_columns`` ``maxColumns``. The key of a value that
    is ``None`` is not written.

    Raises ``TypeError`` for a value JSON cannot write, ``ValueError`` for a
    NaN, an infinity, a string holding a lone surrogate, a cyclic value, an
    ``int`` beyond what a double holds (also inside a list, a tuple or a dict), or
    JSON of more than 2 MiB, and ``RecursionError`` for dicts and lists
    nested past Python's recursion limit.
    """
    sheet_key = "sheetIndex" if isinstance(sheet, int) and not isinstance(sheet, bool) else "sheet"
    entries = {
        sheet_key: ("sheet", sheet),
        "range": ("range", range_),
        "maxRows": ("max_rows", max_rows),
        "maxColumns": ("max_columns", max_columns),
    }
    given = {key: entry for key, entry in entries.items() if entry[1] is not None}
    json = rules_json({key: value for key, (_, value) in given.items()})
    for keyword, value in given.values():
        _within_double(keyword, value)
    return _sized(json)


# The engine reads a JSON number as a double, and takes one beyond a
# double's range as no JSON at all.
def _within_double(keyword: str, value: Any) -> None:
    if isinstance(value, (list, tuple)):
        for item in value:
            _within_double(keyword, item)
    elif isinstance(value, dict):
        for item in value.values():
            _within_double(keyword, item)
    elif isinstance(value, int) and not isinstance(value, bool) and abs(value) > sys.float_info.max:
        digits = len(str(abs(value)))
        message = f"{keyword}: an integer of {digits} digits is beyond what the engine reads"
        raise ValueError(message)


def _sized(json: str) -> bytes:
    data = json.encode("ascii")
    if len(data) > MAX_BYTES:
        message = f"the options are {len(data)} bytes of JSON; the engine takes {MAX_BYTES}"
        raise ValueError(message)
    return data
