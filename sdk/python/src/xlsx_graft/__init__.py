"""Python bindings for xlsx-graft, an engine that reads, verifies, creates and
edits xlsx workbooks.

The engine is a shared C library loaded through ctypes; nothing here
reimplements it. Calls return a :class:`Result` rather than raising. What
raises is a setup with no engine library in it (:class:`LibraryNotFoundError`)
and a book used outside its ``with`` block (:class:`ClosedError`).

Example::

    result = xlsx_graft.version()
    if result.success:
        print(result.value)

Reading a sheet's rows (:class:`XlsxReader`)::

    with xlsx_graft.XlsxReader.open("book.xlsx") as book:
        for row in book.sheet("Data").rows():
            print(row.number, row.values)
    print(book.result.success)
"""

from __future__ import annotations

from xlsx_graft import version_call
from xlsx_graft._version import __version__
from xlsx_graft.book import Book
from xlsx_graft.cell_values import CellError, CellValue
from xlsx_graft.diagnostic import Diagnostic
from xlsx_graft.errors import ClosedError, Error, LibraryNotFoundError
from xlsx_graft.library import Library
from xlsx_graft.result import Result
from xlsx_graft.row import Row, SheetInfo
from xlsx_graft.sheet import Sheet
from xlsx_graft.xlsx_reader import XlsxReader

__all__ = [
    "Book",
    "CellError",
    "CellValue",
    "ClosedError",
    "Diagnostic",
    "Error",
    "Library",
    "LibraryNotFoundError",
    "Result",
    "Row",
    "Sheet",
    "SheetInfo",
    "XlsxReader",
    "__version__",
    "version",
]


def version(*, library: str | None = None) -> Result[str]:
    """The engine's version, such as ``"0.0.0"``, on success.

    ``library`` is the engine library's path, used when ``XLSX_GRAFT_LIBRARY``
    is not set. Raises :class:`LibraryNotFoundError` when no engine library is
    found.
    """
    return version_call.call(Library.open(path=library))
