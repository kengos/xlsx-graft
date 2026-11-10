"""hatchling's build hook: tags the wheel for the platform of its engine.

``scripts/release/assemble-python.sh`` sets ``XLSX_GRAFT_WHEEL_PLATFORM`` to
the wheel platform tag of one platform (``scripts/release/platforms.tsv``) when
it builds a platform wheel, which carries the engine library in
``xlsx_graft/native/``. Without the variable the wheel is ``py3-none-any``, the
wheel for every platform, which carries no engine. The Python code is the same
in both, and ctypes loads the library without a Python extension module, so the
Python and ABI tags are always ``py3`` and ``none``.
"""

from __future__ import annotations

import os
from typing import Any

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


class PlatformTagHook(BuildHookInterface):  # type: ignore[type-arg]
    """Sets the wheel's tag from ``XLSX_GRAFT_WHEEL_PLATFORM``."""

    PLUGIN_NAME = "custom"

    # The signature is hatchling's: `version` is the one it passes and this hook
    # does not need.
    def initialize(self, version: str, build_data: dict[str, Any]) -> None:  # noqa: ARG002
        platform = os.environ.get("XLSX_GRAFT_WHEEL_PLATFORM", "")
        if platform:
            build_data["pure_python"] = False
            build_data["tag"] = f"py3-none-{platform}"
