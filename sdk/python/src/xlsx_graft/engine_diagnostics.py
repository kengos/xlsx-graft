"""Reads the JSON ``xlsx_graft_book_diagnostics`` writes,
``{"has_errors": B, "error": E, "diagnostics": [...]}``.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any

from xlsx_graft import status_diagnostic
from xlsx_graft.diagnostic import Diagnostic

# What reading JSON not of that shape raises (text not UTF-8 too: a
# UnicodeDecodeError is a ValueError).
UNREADABLE = (ValueError, KeyError, TypeError)


@dataclass(frozen=True)
class Report:
    """What the book said: whether a diagnostic is an error, its first
    failure (a :class:`Diagnostic`, or ``None``), and its diagnostics.
    """

    has_errors: bool
    error: Diagnostic | None
    diagnostics: tuple[Diagnostic, ...]


# What a book whose diagnostics could not be read reports: nothing but the
# package's failure.
UNREAD = Report(has_errors=False, error=None, diagnostics=())


def call(text: str) -> Report:
    """Raises one of :data:`UNREADABLE` for JSON not of that shape."""
    report = json.loads(text)
    error = report["error"]
    return Report(
        has_errors=bool(report["has_errors"]),
        error=None if error is None else status_diagnostic.of(error["code"], error["message"]),
        diagnostics=tuple(_of(record) for record in report["diagnostics"]),
    )


def _of(record: dict[str, Any]) -> Diagnostic:
    return Diagnostic(
        code=record["code"],
        severity=record["severity"],
        message=record["message"],
        args=record["args"],
        location=record["location"],
        decided_by=record["decidedBy"],
        fix=record["fix"],
    )
