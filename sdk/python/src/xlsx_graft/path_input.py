"""A path the caller gave, as the C ABI can take it: a path whose bytes are
UTF-8 goes as the path itself, and a path whose bytes are not UTF-8 goes as
the bytes of the file this package reads, since the engine takes no path that
is not UTF-8.
"""

from __future__ import annotations

import os
from collections.abc import Callable
from pathlib import Path
from typing import TypeVar

from xlsx_graft import status_diagnostic
from xlsx_graft.result import Result

T = TypeVar("T")


def call(
    path: str | bytes | os.PathLike[str] | os.PathLike[bytes],
    run: Callable[[str, bytes], Result[T]],
) -> Result[T]:
    """Calls ``run(how, data)`` and returns what it returns: ``how`` is
    ``"path"`` and ``data`` the path's UTF-8 bytes, or ``"bytes"`` and
    ``data`` the file's bytes when the path is not UTF-8. A file that cannot
    be read is an ``io`` failure (``run`` is not called).

    Raises ``TypeError`` for a path of the wrong kind.
    """
    encoded = os.fsencode(path)
    try:
        encoded.decode("utf-8")
    except UnicodeDecodeError:
        return _read(encoded, run)
    return run("path", encoded)


# The engine takes no path that is not UTF-8, so this package reads the file
# and runs on its bytes; a file that cannot be read is an ``io`` failure.
def _read(path: bytes, run: Callable[[str, bytes], Result[T]]) -> Result[T]:
    try:
        data = Path(os.fsdecode(path)).read_bytes()
    except OSError as error:
        message = f"the file could not be read: {error}"
        return Result.failure((), status_diagnostic.of("io", message))
    return run("bytes", data)
