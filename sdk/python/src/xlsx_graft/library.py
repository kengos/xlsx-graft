"""Finds and opens the engine's shared library (libxlsx_graft_capi).

Looked up in this order, and the first one set wins (a set path that does not
load raises rather than falls through):

1. the ``XLSX_GRAFT_LIBRARY`` environment variable
2. the ``path`` argument
3. the copy shipped inside the wheel, under ``xlsx_graft/native/``

The environment comes first because where the engine lives is the
deployment's decision, and it has to be able to override application code.
Nothing here downloads anything.
"""

from __future__ import annotations

import ctypes
import os
from pathlib import Path
from typing import TYPE_CHECKING, Any

from xlsx_graft.errors import LibraryNotFoundError

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

ENV_VAR = "XLSX_GRAFT_LIBRARY"

# The file name on each platform. Windows has no `lib` prefix.
NAMES = ("libxlsx_graft_capi.so", "libxlsx_graft_capi.dylib", "xlsx_graft_capi.dll")

PACKAGED_DIR = Path(__file__).parent / "native"


class Library:
    """One opened engine library."""

    def __init__(self, path: str, handle: ctypes.CDLL) -> None:
        self.path = path
        self._handle = handle

    @classmethod
    def open(cls, *, path: str | None = None, env: Mapping[str, str] = os.environ) -> Library:
        """The library ``env`` or ``path`` names, or the packaged copy.

        Raises :class:`LibraryNotFoundError` when no library is found or it
        cannot be loaded.
        """
        # An empty value counts as unset: dlopen("") would open the running
        # program itself rather than fail.
        found = env.get(ENV_VAR) or path or _packaged()
        if not found:
            raise LibraryNotFoundError(_install_hint("no engine library was found"))

        return cls(found, _handle(found))

    def function(self, name: str, argtypes: Sequence[Any], restype: Any) -> Any:
        """A function the library exports.

        The types are always given: ctypes' default return type is a C
        ``int``, which would cut a pointer in half. Raises
        :class:`LibraryNotFoundError` when the library does not export
        ``name``.
        """
        # Indexing, not getattr: getattr caches one function object per name,
        # so a second declaration would retype the first one's.
        try:
            entry = self._handle[name]
        except AttributeError as error:
            message = f"{self.path} exports no `{name}` ({error})"
            raise LibraryNotFoundError(message) from error

        entry.argtypes = list(argtypes)
        entry.restype = restype
        return entry


def _packaged() -> str | None:
    for name in NAMES:
        candidate = PACKAGED_DIR / name
        if candidate.is_file():
            return str(candidate)
    return None


# CDLL, not PyDLL: CDLL releases the GIL around every call. Never closed:
# dlclose of a Rust library that has run thread-local destructors is not safe,
# and ctypes does not close one either.
def _handle(path: str) -> ctypes.CDLL:
    try:
        return ctypes.CDLL(path)
    except OSError as error:
        message = _install_hint(f"{path} could not be loaded ({error})")
        raise LibraryNotFoundError(message) from error


def _install_hint(reason: str) -> str:
    return (
        f"{reason}.\n\n"
        "This package never downloads the engine. Point it at one of these ways:\n"
        f"  * set {ENV_VAR} to the path of a libxlsx_graft_capi library\n"
        '  * pass library="/path/to/libxlsx_graft_capi.so"\n'
        f"  * put the library in {PACKAGED_DIR}"
    )
