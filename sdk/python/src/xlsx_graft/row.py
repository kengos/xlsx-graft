"""A row of a sheet, and a sheet as the workbook lists it."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from xlsx_graft.cell_values import CellValue


@dataclass(frozen=True)
class Row:
    """One row a sheet writes: its number, from 1, and the value of each
    column from A to its last cell, ``None`` where a column has none. The
    values are those :mod:`xlsx_graft.cell_values` documents.
    """

    number: int
    values: tuple[CellValue, ...]


@dataclass(frozen=True)
class SheetInfo:
    """A sheet as the workbook lists it, from :meth:`Book.sheets
    <xlsx_graft.Book.sheets>`.

    ``index`` counts from 0 and takes in hidden sheets and chartsheets, as
    :meth:`Book.sheet <xlsx_graft.Book.sheet>` does. ``kind`` is
    ``"worksheet"`` (the one kind with rows to read), ``"chartsheet"``,
    ``"dialogsheet"``, ``"macrosheet"`` or ``"unknown"`` (its part is not
    known; the diagnostics say why). ``state`` is ``"visible"``,
    ``"hidden"`` or ``"veryHidden"`` (only a program can show it again). A
    kind or state newer than the package is ``"unknown"``.
    """

    index: int
    name: str
    kind: str
    state: str
