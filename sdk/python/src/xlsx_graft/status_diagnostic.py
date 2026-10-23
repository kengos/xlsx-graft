"""Turns a C ABI status other than ``XLSX_GRAFT_OK`` into a :class:`Diagnostic`.

The codes are provisional: they name the status codes of
engine/capi/include/xlsx_graft.h until the engine reports diagnostics of its
own, which replace them.
"""

from __future__ import annotations

from xlsx_graft.diagnostic import Diagnostic

OK = 0

CODES = {
    1: ("null_arg", "the engine was passed a null pointer"),
    2: ("panic", "the engine panicked; the call did not complete"),
}


def call(status: int) -> Diagnostic:
    """The diagnostic for ``status``, the status a C ABI function returned."""
    unknown = f"the engine returned status {status}, which this package does not know"
    code, message = CODES.get(status, ("unknown_status", unknown))
    return Diagnostic(code=code, message=message)
