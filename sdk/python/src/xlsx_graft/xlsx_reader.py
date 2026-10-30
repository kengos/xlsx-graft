"""Opens a workbook to read its sheets' rows, through the C ABI."""

from __future__ import annotations

import os

from xlsx_graft.book import Book
from xlsx_graft.capi import Capi
from xlsx_graft.library import Library


class XlsxReader:
    """Gives a :class:`~xlsx_graft.Book` to read in a ``with`` block.

    Example::

        with XlsxReader.open("book.xlsx") as book:
            for row in book.sheet("Data").rows():
                print(row.number, row.values)
        for d in book.result.diagnostics:
            print(f"{d.severity} {d.code}: {d.message}", file=sys.stderr)
        if book.result.error:
            print(book.result.error.message, file=sys.stderr)
    """

    @staticmethod
    def open(
        path: str | bytes | os.PathLike[str] | os.PathLike[bytes], *, library: str | None = None
    ) -> Book:
        """The workbook at ``path``, opened as the ``with`` block starts.

        ``library`` is the engine library's path, used when
        ``XLSX_GRAFT_LIBRARY`` is not set. Raises
        :class:`~xlsx_graft.LibraryNotFoundError` when no engine library is
        found, and ``TypeError`` for a path of the wrong kind. The path goes
        to the engine as the file system names it (``os.fsencode``), and one
        that is not UTF-8 is refused (``invalid_arg``): read that file and
        open its bytes.
        """
        capi = Capi(Library.open(path=library))
        encoded = os.fsencode(path)
        return Book(capi, lambda out: capi.book_open_path(encoded, out))

    @staticmethod
    def open_bytes(data: bytes | bytearray | memoryview, *, library: str | None = None) -> Book:
        """As :meth:`open`, the workbook being the bytes of ``data``, copied
        now: changing ``data`` after changes nothing.

        Raises ``TypeError`` for what is not bytes-like (anything ``memoryview``
        takes is read).
        """
        capi = Capi(Library.open(path=library))
        copy = memoryview(data).tobytes()
        return Book(capi, lambda out: capi.book_open_bytes(copy, out))
