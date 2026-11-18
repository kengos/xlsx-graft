"""Turns what a view function returned into a :class:`~xlsx_graft.Result`:
its status and, for ``XLSX_GRAFT_OK``, the view's JSON ``{"ok", "error",
"diagnostics", ...}`` followed by the keys of what the view shows.
"""

from __future__ import annotations

import json
from collections.abc import Callable
from typing import Any, TypeVar

from xlsx_graft import engine_diagnostics, status_diagnostic
from xlsx_graft.errors import Error
from xlsx_graft.result import Result

T = TypeVar("T")


def call(status: int, data: bytes, content: Callable[[dict[str, Any]], T]) -> Result[T]:
    """``data`` is the view's JSON for ``XLSX_GRAFT_OK`` and the message of
    ``XLSX_GRAFT_ERR_REQUEST_INVALID`` for that status (empty when it was
    not read), both UTF-8. ``content`` reads the keys of what the view shows
    into its value: :func:`xlsx_graft.view_content.tree` or
    :func:`xlsx_graft.view_content.grid`.

    A successful result when the engine's ``ok`` is true. It holds the value
    whether ``ok`` is true or not; the value is ``None`` only when the
    package failed to get one (a status other than ``XLSX_GRAFT_OK``, or
    JSON it cannot read).

    Raises ``ValueError`` for ``XLSX_GRAFT_ERR_REQUEST_INVALID``, with the
    engine's message, and :class:`~xlsx_graft.Error` for
    ``XLSX_GRAFT_ERR_INVALID_ARG``, which only a call this package built
    wrongly gets.
    """
    if status == status_diagnostic.OK:
        return _parsed(data, content)
    if status == status_diagnostic.REQUEST_INVALID:
        raise ValueError(_refusal(data))
    if status == status_diagnostic.INVALID_ARG:
        message = (
            f"the engine refused a call this package built (status {status}): a bug of the package"
        )
        raise Error(message)
    return Result.failure((), status_diagnostic.call(status))


# The engine's message, or a fixed one when it could not be read.
def _refusal(data: bytes) -> str:
    if data:
        return data.decode("utf-8", errors="replace")
    return (
        f"the engine refused a value of the options (status {status_diagnostic.REQUEST_INVALID}) "
        "and gave no reason"
    )


def _parsed(data: bytes, content: Callable[[dict[str, Any]], T]) -> Result[T]:
    try:
        view = json.loads(data.decode("utf-8"))
        value = content(view)
        diagnostics = tuple(engine_diagnostics.diagnostic(record) for record in view["diagnostics"])
        if view["ok"] is True:
            return Result.ok(value, diagnostics)
        return Result.failure(diagnostics, engine_diagnostics.failure(view["error"]), value)
    except (*engine_diagnostics.UNREADABLE, AttributeError) as error:
        message = f"the engine's result could not be read: {error}"
        return Result.failure((), status_diagnostic.of("diagnostics_unreadable", message))
