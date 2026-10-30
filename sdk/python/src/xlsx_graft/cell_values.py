"""Turns the ``XlsxGraftValue`` array ``xlsx_graft_rows_next`` lends into
Python values, copying everything out: the array and its text live only until
the next call on the rows handle.

========  =====================================================
kind      Python
========  =====================================================
empty     ``None``
string    ``str``
number    ``float``, ``3.0`` included
boolean   ``bool``
date      ``datetime.date``
time      ``datetime.time``
datetime  ``datetime.datetime``, naive
elapsed   ``datetime.timedelta``, not wrapped at a day
error     :class:`CellError`
========  =====================================================

A date, time or date-time is the workbook's wall-clock time with no time
zone, and Python's types hold one as it is: none is applied.
"""

from __future__ import annotations

import datetime
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any, TypeAlias

from xlsx_graft import capi


@dataclass(frozen=True)
class CellError:
    """A cell holding an error value, its text as the workbook writes it
    (``"#N/A"``, ``"#DIV/0!"``). Kept apart from a ``str`` so that a cell
    holding the text ``"#N/A"`` is not read as the error.
    """

    text: str


CellValue: TypeAlias = (
    str
    | float
    | bool
    | datetime.date
    | datetime.time
    | datetime.datetime
    | datetime.timedelta
    | CellError
    | None
)


class RefusedError(Exception):
    """A value of a kind this package does not know (an engine newer than
    the package, ``unknown_value``). It is not guessed at.
    """

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def call(values: Any, count: int) -> tuple[CellValue, ...]:
    """The ``count`` values at ``values``, a pointer to ``XlsxGraftValue``.

    Raises :class:`RefusedError` when a value cannot be given.
    """
    return tuple(_value(values[index]) for index in range(count))


def _date(value: capi.CValue) -> datetime.date:
    return datetime.date(value.year, value.month, value.day)


def _time(value: capi.CValue) -> datetime.time:
    return datetime.time(value.hour, value.minute, value.second, value.millisecond * 1000)


def _datetime(value: capi.CValue) -> datetime.datetime:
    return datetime.datetime.combine(_date(value), _time(value))


KINDS: tuple[Callable[[capi.CValue], CellValue], ...] = (
    lambda _value: None,
    lambda value: capi.text(value.text, value.text_len),
    lambda value: float(value.number),
    lambda value: value.boolean == 1,
    _date,
    _time,
    _datetime,
    lambda value: datetime.timedelta(milliseconds=value.elapsed),
    lambda value: CellError(capi.text(value.text, value.text_len)),
)


def _value(value: capi.CValue) -> CellValue:
    kind: int = value.kind
    if kind >= len(KINDS):
        message = f"the engine returned a value of kind {kind}, which this package does not know"
        raise RefusedError("unknown_value", message)
    return KINDS[kind](value)
