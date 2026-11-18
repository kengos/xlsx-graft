"""What :meth:`Viewer.describe_sheet <xlsx_graft.Viewer.describe_sheet>`
shows: the cells of one sheet. Each class is one object of the engine's JSON
and each attribute one of its keys. Text is made for display (control
characters removed, long text cut), so it is not exact.
"""

from __future__ import annotations

from dataclasses import dataclass

from xlsx_graft.row import SheetInfo


@dataclass(frozen=True)
class GridTruncated:
    """Whether the rows, the columns or the merges of the sheet go past what
    is shown.
    """

    rows: bool
    columns: bool
    merges: bool


@dataclass(frozen=True)
class GridCell:
    """One cell: its ``ref`` (``A1``), its ``type`` (``string``, ``number``,
    ``boolean``, ``date``, ``time``, ``datetime``, ``elapsed`` or
    ``error``), its ``value`` as the engine's JSON gives it (a ``str``, a
    ``float``, an ``int`` of milliseconds for an elapsed time, or a
    ``bool``; a date, a time and a date-time are text) and whether its text
    was cut (``truncated``).
    """

    ref: str
    type: str
    value: str | float | int | bool
    truncated: bool


@dataclass(frozen=True)
class GridRow:
    """One row shown: its number from 1, and its ``cells`` (the cells the
    sheet writes).
    """

    row: int
    cells: tuple[GridCell, ...]


@dataclass(frozen=True)
class Grid:
    """The sheet shown (``None`` when it was not found), the ranges
    ``dimension`` (the sheet's own claim), ``used`` (where cells are) and
    ``shown`` (what the rows hold), what is ``truncated``, whether the view
    is ``complete``, its ``rows`` and the ``merges`` that are shown.
    """

    sheet: SheetInfo | None
    dimension: str | None
    used: str | None
    shown: str | None
    truncated: GridTruncated
    complete: bool
    rows: tuple[GridRow, ...]
    merges: tuple[str, ...]
