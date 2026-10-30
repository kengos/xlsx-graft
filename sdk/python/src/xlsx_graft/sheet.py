"""One sheet of a :class:`~xlsx_graft.Book`, whose rows :meth:`Sheet.rows`
reads.
"""

from __future__ import annotations

import ctypes
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Generator

    from xlsx_graft.book import Book
    from xlsx_graft.row import Row

# The largest index a size_t holds.
MAX_INDEX = 2 ** (8 * ctypes.sizeof(ctypes.c_size_t)) - 1


class Sheet:
    """A sheet asked for by name or index, from :meth:`Book.sheet
    <xlsx_graft.Book.sheet>`.
    """

    def __init__(self, book: Book, which: str | int) -> None:
        # bool is an int, and no index.
        if isinstance(which, bool) or not isinstance(which, str | int):
            raise TypeError(f"a sheet is a name or an index from 0, not {which!r}")
        if isinstance(which, int) and not 0 <= which <= MAX_INDEX:
            raise ValueError(f"a sheet is a name or an index from 0, not {which!r}")
        self._book = book
        self._which = which

    def rows(self) -> Generator[Row, None, None]:
        """Each row the sheet writes, as a :class:`~xlsx_graft.Row`, in
        order, read one at a time: no more of the sheet is held than the row
        given. Only the rows the sheet writes come. Each call reads the sheet
        from its start, when the first row is asked for.

        Reading stops at the first problem in the sheet, the rows before it
        having come, and gives nothing when the engine refuses the sheet (no
        such sheet, not a worksheet); either way the book's
        :attr:`~xlsx_graft.Book.result` fails, saying why. A generator left
        unfinished holds its rows until it is closed (``close()``) or
        collected, or the book's block ends.

        The generator raises :class:`~xlsx_graft.ClosedError` when a row is
        asked of it outside the book's ``with`` block.
        """
        rows = self._book.open_rows(self._which)
        if rows is None:
            return
        try:
            while (row := self._book.next_row(rows)) is not None:
                yield row
        finally:
            self._book.close_rows(rows)
