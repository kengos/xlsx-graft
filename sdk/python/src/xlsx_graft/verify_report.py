"""Turns what a verify function returned into a :class:`~xlsx_graft.Result`:
its status and, for ``XLSX_GRAFT_OK``, the result's JSON
``{"ok", "failOn", "counts", "error", "diagnostics"}``.
"""

from __future__ import annotations

import json

from xlsx_graft import engine_diagnostics, status_diagnostic
from xlsx_graft.errors import Error
from xlsx_graft.result import Result


def call(status: int, data: bytes) -> Result[None]:
    """``data`` is the result's JSON as the engine wrote it, UTF-8.

    A successful result with no value when ``ok`` is true. ``counts`` and
    ``failOn`` are not kept: the counts are those of ``diagnostics`` by
    severity. A status other than ``XLSX_GRAFT_OK`` is a failure whose
    ``error`` names it.

    Raises :class:`~xlsx_graft.Error` for ``XLSX_GRAFT_ERR_INVALID_ARG``,
    which only a call this package built wrongly gets.
    """
    if status == status_diagnostic.OK:
        return _parsed(data)
    if status == status_diagnostic.INVALID_ARG:
        message = (
            f"the engine refused a call this package built (status {status}): a bug of the package"
        )
        raise Error(message)
    return Result.failure((), status_diagnostic.call(status))


def _parsed(data: bytes) -> Result[None]:
    try:
        report = json.loads(data.decode("utf-8"))
        records = report["diagnostics"]
        ok = report["ok"]
        if not isinstance(records, list) or not isinstance(ok, bool):
            message = "diagnostics is not a list or ok is not a boolean"
            raise TypeError(message)
        diagnostics = tuple(engine_diagnostics.diagnostic(record) for record in records)
        if ok:
            return Result(success=True, value=None, diagnostics=diagnostics)
        return Result.failure(diagnostics, engine_diagnostics.failure(report["error"]))
    except engine_diagnostics.UNREADABLE as error:
        message = f"the engine's result could not be read: {error}"
        return Result.failure((), status_diagnostic.of("diagnostics_unreadable", message))
