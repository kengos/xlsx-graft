"""One thing the engine reported about a call."""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any

# The args and location of a diagnostic the package makes: it has neither.
NONE: Mapping[str, Any] = MappingProxyType({})


@dataclass(frozen=True)
class Diagnostic:
    """``code`` is the stable, machine-readable part; ``message`` is for people.

    The engine's own diagnostics (:attr:`Result.diagnostics
    <xlsx_graft.Result.diagnostics>` of a read) carry every field of the JSON
    every host prints, keys as written there: ``severity`` (``"error"``,
    ``"warning"`` or ``"notice"``), ``args``, ``location``, ``decided_by``
    (its ``decidedBy``) and ``fix``. One the package makes from a C ABI
    status is an ``"error"`` with ``args`` and ``location`` empty and no
    ``decided_by`` or ``fix``.
    """

    code: str
    severity: str
    message: str
    # Hashed by code, severity and message: the rest holds JSON objects,
    # which do not hash.
    args: Mapping[str, Any] = field(default_factory=lambda: NONE, hash=False)
    location: Mapping[str, Any] = field(default_factory=lambda: NONE, hash=False)
    decided_by: Mapping[str, Any] | None = field(default=None, hash=False)
    fix: Any = field(default=None, hash=False)
