"""One thing the engine reported about a call."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Diagnostic:
    """``code`` is the stable, machine-readable part; ``message`` is for people."""

    code: str
    message: str
