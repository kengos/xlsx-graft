"""A workbook to read, which :class:`~xlsx_graft.XlsxReader` gives."""

from __future__ import annotations

import ctypes
import threading
from typing import TYPE_CHECKING

from xlsx_graft import cell_values, engine_diagnostics, status_diagnostic
from xlsx_graft.errors import ClosedError
from xlsx_graft.result import Result
from xlsx_graft.row import Row, SheetInfo
from xlsx_graft.sheet import Sheet

if TYPE_CHECKING:
    from collections.abc import Callable
    from types import TracebackType

    from xlsx_graft.capi import Capi
    from xlsx_graft.diagnostic import Diagnostic

KINDS = ("worksheet", "chartsheet", "dialogsheet", "macrosheet", "unknown")
STATES = ("visible", "hidden", "veryHidden")
CLOSED = "the book is not open: use it inside its with block"
INTERRUPTED = "an exception cut the book's close short, before it read the diagnostics"


class Book:
    """A workbook open to read inside a ``with`` block: its sheets, and the
    rows of each through :meth:`sheet`.

    The ``with`` statement opens it and closes it when the block ends,
    however it ends, and with it every sheet's rows; using it outside the
    block raises :class:`~xlsx_graft.ClosedError`. Then :attr:`result` says
    how the reading went.

    Nothing here leaves the block early. A file that cannot be opened lists
    no sheets, and a sheet the engine refuses (no such sheet, an unreadable
    workbook) reads no rows; the :attr:`result` fails, saying why, as it does
    when a problem stops a sheet's rows or the host fails. A panic in the
    engine ends every sheet of the book: none reads a row after it, and the
    book lists no sheets. An exception the block raises is the caller's: it
    propagates, the book closed first.

    A book and its rows share state the library does not lock, so every call
    into it, closes included, is made under the book's own lock, and what it
    lends is copied out before the lock is let go.

    Python cannot hold an interrupt off as Ruby does, but one call into the
    library is whole (ctypes gives the interrupt back after it returns). So
    a handle's slot exists before the call that fills it, and a handle leaves
    the book before the call that frees it: an interrupt leaks a handle at
    worst, never frees one twice. The result is a close's that ran whole.
    """

    def __init__(self, capi: Capi, opening: Callable[[ctypes.c_void_p], int]) -> None:
        """:class:`~xlsx_graft.XlsxReader` makes the book; ``opening`` opens
        it, writing its handle into the ``c_void_p`` it is given.
        """
        self._capi = capi
        self._opening = opening
        self._handle = ctypes.c_void_p()
        # Reentrant: a rows generator the garbage collector finalizes frees
        # its handle, on whichever thread it runs, maybe one in a call.
        self._lock = threading.RLock()
        self._entered = False
        self._closed = False
        # The rows handles open, each freed by its sheet's read or at the end.
        self._rows: list[ctypes.c_void_p] = []
        # Why the package stopped short where the engine keeps no failure.
        self._failure: Diagnostic | None = None
        self._result: Result[None] | None = None

    def __enter__(self) -> Book:
        """Opens the workbook. When it cannot be, the block runs all the
        same, with no sheets, and :attr:`result` says why.

        Raises ``ValueError`` for a book entered twice.
        """
        with self._lock:
            if self._entered or self._closed:
                raise ValueError("a book is opened once: call XlsxReader.open again")
            self._entered = True
            # The handle is written into the book's own c_void_p, so an
            # interrupt after the call still finds it to close.
            try:
                status = self._opening(self._handle)
                # Read only on OK, as the header says: a panic may have
                # written half of it.
                if status != status_diagnostic.OK:
                    self._handle.value = None
                self._failed(status)
            except BaseException:
                self.close()
                raise
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        self.close()

    @property
    def result(self) -> Result[None]:
        """How the reading went, once the book is closed (after its ``with``
        block). Its error is the first failure; the engine's words for it
        when the engine kept it too. The value is always ``None``: keep what
        you read in a variable outside the block.

        A close an exception (an interrupt) cut short read no diagnostics:
        its result fails with ``interrupted``, never a success.

        Raises ``ValueError`` before the book is closed.
        """
        # Locked: a close under way on another thread has not set it yet.
        with self._lock:
            if not self._closed:
                raise ValueError(
                    "the result is known once the book is closed, after its with block"
                )
            if self._result is None:
                return Result.failure((), status_diagnostic.of("interrupted", INTERRUPTED))
            return self._result

    def sheets(self) -> list[SheetInfo]:
        """The sheets, in the order the workbook lists them, hidden ones
        included; none when the workbook cannot be read.
        """
        with self._lock:
            self._check_open()
            status, sheets = self._capi.book_sheets(self._handle)
            self._failed(status)
        return [
            SheetInfo(
                index=index,
                name=name,
                kind=KINDS[kind] if kind < len(KINDS) else "unknown",
                state=STATES[state] if state < len(STATES) else "unknown",
            )
            for index, (name, kind, state) in enumerate(sheets)
        ]

    def sheet(self, which: str | int = 0) -> Sheet:
        """The sheet named ``which`` (letter case aside) or at index ``which``
        (from 0, hidden sheets and chartsheets counted); the first when not
        given. Nothing is read until :meth:`Sheet.rows
        <xlsx_graft.Sheet.rows>`. A name or an index no sheet has is the
        engine's to refuse there; to look for a sheet, look in
        :meth:`sheets`: a refused sheet fails the :attr:`result`.

        Raises ``TypeError`` for neither a ``str`` nor an ``int``, and
        ``ValueError`` for an index below 0 or past what a ``size_t`` holds.
        """
        with self._lock:
            self._check_open()
        return Sheet(self, which)

    def close(self) -> None:
        """Reads the diagnostics, then closes the book, which closes its
        rows, and frees what is left of each rows handle. Once: a second
        close does nothing. The ``with`` block calls it.
        """
        with self._lock:
            if self._closed:
                return
            self._closed = True
            try:
                report = self._report()
            finally:
                self._free()
            # The last statement: a close an exception cuts short leaves no
            # result, which :attr:`result` reads as ``interrupted``.
            self._result = self._outcome(report)

    # What Sheet calls.

    def open_rows(self, which: str | int) -> ctypes.c_void_p | None:
        """The rows handle, ``None`` when the sheet was refused."""
        with self._lock:
            self._check_open()
            # Listed before the call, so an interrupt after it still finds
            # the handle to free; a NULL one frees nothing.
            rows = ctypes.c_void_p()
            self._rows.append(rows)
            if isinstance(which, str):
                name = which.encode("utf-8", "surrogatepass")
                status = self._capi.rows_open_name(self._handle, name, rows)
            else:
                status = self._capi.rows_open(self._handle, which, rows)
            self._failed(status)
            if status == status_diagnostic.OK:
                return rows
            self._rows.remove(rows)
            return None

    def next_row(self, rows: ctypes.c_void_p) -> Row | None:
        """The next row; ``None`` at the end of the sheet, or when reading it
        stopped short, which the :attr:`result` says.
        """
        with self._lock:
            self._check_open()
            status, number, values, count = self._capi.rows_next(rows)
            self._failed(status)
            if status != status_diagnostic.OK or number == 0:
                return None
            try:
                return Row(number=number, values=cell_values.call(values, count))
            except cell_values.RefusedError as error:
                self._fail_with(status_diagnostic.of(error.code, f"row {number}: {error}"))
                return None

    def close_rows(self, rows: ctypes.c_void_p) -> None:
        """Frees a rows handle :meth:`open_rows` made, unless the book's close
        did.
        """
        with self._lock:
            if rows in self._rows:
                self._rows.remove(rows)
                self._failed(self._capi.rows_close(rows))
                rows.value = None

    def _free(self) -> None:
        """Closes the book, which closes its rows, and frees what is left of
        each rows handle. Each leaves the book before its call frees it: an
        exception between two lines leaks at worst, never frees twice.
        """
        self._failed(self._capi.book_close(self._handle))
        self._handle.value = None
        while self._rows:
            self._failed(self._capi.rows_close(self._rows.pop()))

    def _check_open(self) -> None:
        if not self._entered or self._closed:
            raise ClosedError(CLOSED)

    def _failed(self, status: int) -> None:
        """Keeps ``status`` as the book's failure unless it is OK or one is
        kept. The host's reason is read at once: it is the thread's, and the
        next failure on the thread replaces it.
        """
        if status == status_diagnostic.OK:
            return
        message = self._capi.error_message() if status == status_diagnostic.IO else None
        self._fail_with(status_diagnostic.call(status, message))

    def _fail_with(self, diagnostic: Diagnostic) -> None:
        if self._failure is None:
            self._failure = diagnostic

    def _report(self) -> engine_diagnostics.Report:
        """The book's diagnostics; none when they cannot be read. A book that
        did not open has none, its handle NULL, which the library refuses.
        """
        try:
            status, text = self._capi.book_diagnostics(self._handle)
            if status != status_diagnostic.OK:
                self._failed(status)
                return engine_diagnostics.UNREAD
            return engine_diagnostics.call(text)
        except engine_diagnostics.UNREADABLE as error:
            message = f"the engine's diagnostics could not be read: {error}"
            self._fail_with(status_diagnostic.of("diagnostics_unreadable", message))
            return engine_diagnostics.UNREAD

    def _outcome(self, report: engine_diagnostics.Report) -> Result[None]:
        """The package's first failure, or the engine's: the engine's words
        for it when the engine kept the same.
        """
        error = report.error
        if self._failure is not None and (error is None or self._failure.code != error.code):
            error = self._failure
        if error is not None or report.has_errors:
            return Result.failure(report.diagnostics, error)
        return Result.ok(None, report.diagnostics)
