from __future__ import annotations

import pytest

import xlsx_graft
from xlsx_graft.library import Library


def test_version_returns_the_engines_version_from_the_library(engine_version: str) -> None:
    result = xlsx_graft.version()
    assert result == xlsx_graft.Result(success=True, value=engine_version, diagnostics=())
    assert not result.failed


def test_the_package_version_tracks_the_engines(engine_version: str) -> None:
    assert xlsx_graft.__version__ == engine_version


def test_version_passes_library_on_to_library_open(
    monkeypatch: pytest.MonkeyPatch, engine_library: str
) -> None:
    paths: list[str | None] = []
    original = Library.open

    def spy(*, path: str | None = None) -> Library:
        paths.append(path)
        return original(path=engine_library)

    monkeypatch.setattr(Library, "open", spy)
    xlsx_graft.version(library="/elsewhere/libxlsx_graft_capi.so")
    assert paths == ["/elsewhere/libxlsx_graft_capi.so"]
