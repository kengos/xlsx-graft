"""Python bindings for xlsx-graft, an engine that reads, verifies, creates and
edits xlsx workbooks.

The engine is a shared C library loaded through ctypes; nothing here
reimplements it. Calls return a :class:`Result` rather than raising. What
raises is a setup with no engine library in it (:class:`LibraryNotFoundError`),
a policy the engine does not take (:class:`PolicyInvalidError`), a book used
outside its ``with`` block (:class:`ClosedError`) and a value of
:meth:`Viewer.describe_sheet` that the engine refuses (``ValueError``).

Example::

    result = xlsx_graft.version()
    if result.success:
        print(result.value)

Reading a sheet's rows (:class:`XlsxReader`)::

    with xlsx_graft.XlsxReader.open("book.xlsx") as book:
        for row in book.sheet("Data").rows():
            print(row.number, row.values)
    print(book.result.success)

Verifying a workbook (:class:`Verifier`)::

    import sys

    verifier = xlsx_graft.Verifier(policy={"max_formula_bytes": 32768})
    result = verifier.verify("book.xlsx", targets=["excel >= 2016"])
    for d in result.diagnostics:
        print(f"{d.severity} {d.code}: {d.message}", file=sys.stderr)

Viewing a workbook (:class:`Viewer`)::

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

from __future__ import annotations

from xlsx_graft import version_call
from xlsx_graft._version import __version__
from xlsx_graft.book import Book
from xlsx_graft.cell_values import CellError, CellValue
from xlsx_graft.diagnostic import Diagnostic
from xlsx_graft.errors import ClosedError, Error, LibraryNotFoundError, PolicyInvalidError
from xlsx_graft.grid import Grid, GridCell, GridRow, GridTruncated
from xlsx_graft.library import Library
from xlsx_graft.result import Result
from xlsx_graft.row import Row, SheetInfo
from xlsx_graft.sheet import Sheet
from xlsx_graft.tree import Tree, TreePart, TreeRelationship, TreeTarget
from xlsx_graft.verifier import Verifier
from xlsx_graft.viewer import Viewer
from xlsx_graft.xlsx_reader import XlsxReader

__all__ = [
    "Book",
    "CellError",
    "CellValue",
    "ClosedError",
    "Diagnostic",
    "Error",
    "Grid",
    "GridCell",
    "GridRow",
    "GridTruncated",
    "Library",
    "LibraryNotFoundError",
    "PolicyInvalidError",
    "Result",
    "Row",
    "Sheet",
    "SheetInfo",
    "Tree",
    "TreePart",
    "TreeRelationship",
    "TreeTarget",
    "Verifier",
    "Viewer",
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
