"""Turns a C ABI status other than ``XLSX_GRAFT_OK`` into a :class:`Diagnostic`,
for a status the engine keeps no failure of: one with no book to keep it (an
open), or after which its diagnostics cannot be read (a panic).

The statuses that refuse a sheet are named as ``xlsx-graft read`` names them
in its end line's ``error.code``; the book's diagnostics say them in full,
naming the sheet. The others name the status codes of
engine/capi/include/xlsx_graft.h.
"""

from __future__ import annotations

from xlsx_graft.diagnostic import Diagnostic

OK = 0
PANIC = 2
IO = 3

CODES = {
    1: ("null_arg", "the engine was passed a null pointer"),
    PANIC: ("panic", "the engine panicked; the call did not complete"),
    IO: ("io", "the host failed to open, read or write a file"),
    4: ("invalid_arg", "the engine was passed text that is not UTF-8"),
    5: ("workbook_unreadable", "the workbook cannot be read; the diagnostics say why"),
    6: ("sheet_not_found", "no such sheet"),
    7: ("sheet_ambiguous", "more than one sheet has the name, letter case aside"),
    8: ("sheet_not_worksheet", "the sheet is not a worksheet"),
    9: ("sheet_unreadable", "the sheet cannot be read; the diagnostics say why"),
    10: ("closed", "the rows' book was closed"),
}


def call(status: int, message: str | None = None) -> Diagnostic:
    """The diagnostic for ``status``, the status a C ABI function returned.

    ``message`` is what to say in place of the status's own message, for one
    that knows more (why the host failed).
    """
    unknown = f"the engine returned status {status}, which this package does not know"
    code, default = CODES.get(status, ("unknown_status", unknown))
    return of(code, message or default)


def of(code: str, message: str) -> Diagnostic:
    """An ``"error"`` diagnostic of the package's own, not from the engine's
    diagnostics.
    """
    return Diagnostic(code=code, severity="error", message=message)
