from __future__ import annotations

import ctypes
import shutil
from typing import TYPE_CHECKING

import pytest

from xlsx_graft import LibraryNotFoundError, library
from xlsx_graft.library import Library

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def packaged_dir(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    directory = tmp_path / "native"
    directory.mkdir()
    monkeypatch.setattr(library, "PACKAGED_DIR", directory)
    return directory


@pytest.mark.usefixtures("packaged_dir")
class TestOpen:
    def test_takes_xlsx_graft_library_over_the_path_argument(self, engine_library: str) -> None:
        env = {"XLSX_GRAFT_LIBRARY": engine_library}
        assert Library.open(path="/nowhere.so", env=env).path == engine_library

    def test_takes_the_path_argument_when_xlsx_graft_library_is_empty(
        self, engine_library: str
    ) -> None:
        opened = Library.open(path=engine_library, env={"XLSX_GRAFT_LIBRARY": ""})
        assert opened.path == engine_library

    def test_falls_back_to_the_packaged_copy(self, engine_library: str, packaged_dir: Path) -> None:
        shutil.copy(engine_library, packaged_dir / "libxlsx_graft_capi.so")
        opened = Library.open(path="", env={})
        assert opened.path == str(packaged_dir / "libxlsx_graft_capi.so")

    def test_raises_with_the_ways_to_install_when_nothing_is_found(self) -> None:
        hint = r"(?s)no engine library was found.*XLSX_GRAFT_LIBRARY"
        with pytest.raises(LibraryNotFoundError, match=hint):
            Library.open(env={})

    def test_raises_naming_the_file_when_it_is_not_a_library(self, tmp_path: Path) -> None:
        not_a_library = tmp_path / "not_a_library.so"
        not_a_library.write_text("text", encoding="utf-8")
        with pytest.raises(LibraryNotFoundError, match=r"not_a_library\.so could not be loaded"):
            Library.open(path=str(not_a_library), env={})


def test_function_raises_when_the_library_does_not_export_the_name(engine_library: str) -> None:
    opened = Library.open(env={"XLSX_GRAFT_LIBRARY": engine_library})
    with pytest.raises(LibraryNotFoundError, match="exports no `xlsx_graft_nothing`"):
        opened.function("xlsx_graft_nothing", [], ctypes.c_int32)
