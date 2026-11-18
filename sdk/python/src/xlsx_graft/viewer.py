"""Shows a workbook's parts and a sheet's cells through the C ABI: the
engine's ``tree`` and ``describe_sheet`` views (docs/sdk.md, "Views in an
SDK").
"""

from __future__ import annotations

import os
from collections.abc import Callable
from typing import Any, TypeVar

from xlsx_graft import path_input, view_content, view_options, view_report
from xlsx_graft.grid import Grid
from xlsx_graft.library import Library
from xlsx_graft.result import Result
from xlsx_graft.tree import Tree
from xlsx_graft.view_call import ViewCall

T = TypeVar("T")


class Viewer:
    """Shows a workbook's parts and a sheet's cells. The limits a workbook is
    read under are the CLI's defaults; no policy is taken.

    A viewer does not change once it is made, so one may be shared by any
    number of threads, and their calls may run at once.

    Example::

        viewer = xlsx_graft.Viewer()
        tree = viewer.tree("book.xlsx").value
        if tree:
            for part in tree.parts:
                print(part.name, part.size)
        result = viewer.describe_sheet("book.xlsx", sheet="Data", range="A1:H50", max_rows=20)
        if result.value is not None:
            for row in result.value.rows:
                print([cell.value for cell in row.cells])
    """

    def __init__(self, *, library: str | None = None) -> None:
        """``library`` is the engine library's path, used when
        ``XLSX_GRAFT_LIBRARY`` is not set.

        Raises :class:`~xlsx_graft.LibraryNotFoundError` when no engine
        library is found.
        """
        self._call = ViewCall(Library.open(path=library))

    def tree(self, path: str | bytes | os.PathLike[str] | os.PathLike[bytes]) -> Result[Tree]:
        """The parts of the workbook at ``path`` and the relationships
        between them.

        The result is successful when the engine's ``ok`` is true. Its
        ``value`` is a :class:`~xlsx_graft.Tree` also when it is not (a
        workbook that is broken, or a file that cannot be read, gives a
        ``Tree`` with the parts that were read, perhaps none); it is
        ``None`` only when this package failed to get one (``panic``,
        ``null_arg``, ``unknown_status``, ``diagnostics_unreadable``, or
        ``io`` for a path that is not UTF-8 whose file this package cannot
        read). A file that cannot be opened is an ``io`` ``error``.

        Raises ``TypeError`` for a path of the wrong kind.
        """
        return path_input.call(
            path, lambda how, data: self._run(f"tree_{how}", data, b"", view_content.tree)
        )

    def tree_bytes(self, data: bytes | bytearray | memoryview) -> Result[Tree]:
        """As :meth:`tree`, the workbook being the bytes of ``data``, copied
        now: changing ``data`` during the call changes nothing.

        Raises ``TypeError`` for what is not bytes-like (anything
        ``memoryview`` takes is read).
        """
        return self._run("tree_bytes", memoryview(data).tobytes(), b"", view_content.tree)

    def describe_sheet(
        self,
        path: str | bytes | os.PathLike[str] | os.PathLike[bytes],
        *,
        sheet: str | int | None = None,
        range: str | None = None,  # noqa: A002 (the engine's key, as the other SDKs name it)
        max_rows: int | None = None,
        max_columns: int | None = None,
    ) -> Result[Grid]:
        """The cells of one sheet of the workbook at ``path``.

        ``sheet`` is the sheet's index (an ``int``) or name; ``None`` is the
        first sheet. ``range`` is an A1 range such as ``"A1:H50"``: the cells
        shown start at its top left cell (A1 without it) and never go past
        it. ``max_rows`` is 1 to 10000 (100 without it) and ``max_columns``
        is 1 to 1000 (30 without it).

        The result is successful when the engine's ``ok`` is true. Its
        ``value`` is a :class:`~xlsx_graft.Grid` also when it is not (a
        sheet that is not found gives a ``Grid`` whose ``sheet`` is
        ``None``); it is ``None`` in the cases of :meth:`tree`.

        Raises ``ValueError`` when the engine refuses a value (``range``
        that is no A1 range, ``max_rows`` out of range, a ``sheet`` that is
        neither a name nor an index), with the engine's message. Before the
        call it raises ``ValueError`` for an ``int`` beyond what a double
        holds, a NaN, an infinity, a string holding a lone surrogate or
        options of more than 2 MiB of JSON, ``TypeError`` for a value JSON
        cannot write, and ``RecursionError`` for dicts and lists nested past
        Python's recursion limit; and ``TypeError`` for a path of the wrong
        kind.
        """
        options = view_options.call(sheet, range, max_rows, max_columns)
        return path_input.call(
            path,
            lambda how, data: self._run(f"describe_sheet_{how}", data, options, view_content.grid),
        )

    def describe_sheet_bytes(
        self,
        data: bytes | bytearray | memoryview,
        *,
        sheet: str | int | None = None,
        range: str | None = None,  # noqa: A002 (the engine's key, as the other SDKs name it)
        max_rows: int | None = None,
        max_columns: int | None = None,
    ) -> Result[Grid]:
        """As :meth:`describe_sheet`, the workbook being the bytes of
        ``data``, copied now: changing ``data`` during the call changes
        nothing.

        Raises ``TypeError`` for what is not bytes-like, and as
        :meth:`describe_sheet`.
        """
        options = view_options.call(sheet, range, max_rows, max_columns)
        return self._run(
            "describe_sheet_bytes", memoryview(data).tobytes(), options, view_content.grid
        )

    def _run(
        self, name: str, data: bytes, options: bytes, content: Callable[[dict[str, Any]], T]
    ) -> Result[T]:
        return view_report.call(*self._call.call(name, data, options), content)
