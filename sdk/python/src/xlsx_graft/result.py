"""What every call into the engine returns."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Generic, TypeVar

from xlsx_graft.diagnostic import Diagnostic

T = TypeVar("T")


@dataclass(frozen=True)
class Result(Generic[T]):
    """Whether the call worked, the value when it did, and the diagnostics
    either way. ``value`` is ``None`` on a failure.
    """

    success: bool
    value: T | None
    diagnostics: tuple[Diagnostic, ...] = field(default=())

    @classmethod
    def ok(cls, value: T, diagnostics: tuple[Diagnostic, ...] = ()) -> Result[T]:
        return cls(success=True, value=value, diagnostics=diagnostics)

    @classmethod
    def failure(cls, diagnostics: tuple[Diagnostic, ...]) -> Result[T]:
        return cls(success=False, value=None, diagnostics=diagnostics)

    @property
    def failed(self) -> bool:
        return not self.success
