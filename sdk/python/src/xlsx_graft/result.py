"""What every call into the engine returns."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Generic, TypeVar

from xlsx_graft.diagnostic import Diagnostic

T = TypeVar("T")


@dataclass(frozen=True)
class Result(Generic[T]):
    """Whether the call worked, the value when it did, why it did not when it
    stopped, and the diagnostics either way. ``value`` is ``None`` on a
    failure, except for a call of :class:`Viewer <xlsx_graft.Viewer>`: its
    value is a :class:`Tree <xlsx_graft.Tree>` or a :class:`Grid
    <xlsx_graft.Grid>` also on a failure, and ``None`` only when the package
    itself failed to get one.

    ``error`` is a :class:`Diagnostic` saying why the call stopped short: the
    workbook or a sheet asked for was refused, or the host or the engine
    failed. A call can fail with no ``error`` too: a read that a problem in
    its sheet stopped, whose diagnostics hold an error, as
    ``xlsx-graft read``'s ``ok`` is false then.
    """

    success: bool
    value: T | None
    diagnostics: tuple[Diagnostic, ...] = field(default=())
    error: Diagnostic | None = None

    @classmethod
    def ok(cls, value: T, diagnostics: tuple[Diagnostic, ...] = ()) -> Result[T]:
        return cls(success=True, value=value, diagnostics=diagnostics)

    @classmethod
    def failure(
        cls,
        diagnostics: tuple[Diagnostic, ...],
        error: Diagnostic | None = None,
        value: T | None = None,
    ) -> Result[T]:
        return cls(success=False, value=value, diagnostics=diagnostics, error=error)

    @property
    def failed(self) -> bool:
        return not self.success
