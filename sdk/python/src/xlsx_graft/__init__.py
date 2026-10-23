"""Python bindings for xlsx-graft, an engine that reads, verifies, creates and
edits xlsx workbooks.

The engine is a shared C library loaded through ctypes; nothing here
reimplements it. Calls return a :class:`Result` rather than raising. What
raises is a setup with no engine library in it (:class:`LibraryNotFoundError`).

Example::

    result = xlsx_graft.version()
    if result.success:
        print(result.value)
"""

from __future__ import annotations

from xlsx_graft import version_call
from xlsx_graft._version import __version__
from xlsx_graft.diagnostic import Diagnostic
from xlsx_graft.errors import Error, LibraryNotFoundError
from xlsx_graft.library import Library
from xlsx_graft.result import Result

__all__ = [
    "Diagnostic",
    "Error",
    "Library",
    "LibraryNotFoundError",
    "Result",
    "__version__",
    "version",
]


def version(*, library: str | None = None) -> Result[str]:
    """The engine's version, such as ``"0.0.0"``, on success.

    ``library`` is the engine library's path, used when ``XLSX_GRAFT_LIBRARY``
    is not set. Raises :class:`LibraryNotFoundError` when no engine library is
    found.
    """
    return version_call.call(Library.open(path=library))
