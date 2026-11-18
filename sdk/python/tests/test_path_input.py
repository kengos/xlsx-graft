from __future__ import annotations

import os
from pathlib import Path

import pytest

from xlsx_graft import Result, path_input

TABLE = Path(__file__).resolve().parents[3] / "fixtures" / "table.xlsx"


def yielded(path: str | bytes | os.PathLike[str]) -> Result[tuple[str, bytes]]:
    return path_input.call(path, lambda how, data: Result.ok((how, data)))


def test_runs_with_the_path_as_utf_8_bytes_for_a_path_of_each_kind() -> None:
    expected = Result.ok(("path", os.fsencode(TABLE)))
    assert yielded(TABLE) == expected
    assert yielded(str(TABLE)) == expected
    assert yielded(os.fsencode(TABLE)) == expected


@pytest.mark.parametrize("name", [b"\xff.xlsx", b"\xe9.xlsx"])
def test_runs_with_the_bytes_of_the_file_for_a_path_that_is_not_utf_8(
    tmp_path: Path, name: bytes
) -> None:
    path = tmp_path / os.fsdecode(name)
    path.write_bytes(TABLE.read_bytes())
    assert yielded(path) == Result.ok(("bytes", TABLE.read_bytes()))
    assert yielded(os.fsencode(path)) == Result.ok(("bytes", TABLE.read_bytes()))


def test_returns_an_io_failure_and_does_not_run_for_a_file_that_cannot_be_read(
    tmp_path: Path,
) -> None:
    calls: list[str] = []

    def run(how: str, _data: bytes) -> Result[None]:
        calls.append(how)
        return Result.ok(None)

    result = path_input.call(tmp_path / os.fsdecode(b"\xfe.xlsx"), run)
    assert (result.success, result.diagnostics, calls) == (False, (), [])
    assert result.error is not None
    assert result.error.code == "io"
    assert result.error.message.startswith("the file could not be read: ")


def test_raises_type_error_for_a_path_of_the_wrong_kind() -> None:
    with pytest.raises(TypeError):
        yielded(1)  # type: ignore[arg-type]
