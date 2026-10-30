from __future__ import annotations

from typing import Any

import pytest

from xlsx_graft import Diagnostic, Result, version_call
from xlsx_graft.library import Library


def returning(status: int) -> Any:
    def function(*_args: object) -> int:
        return status

    return lambda *_args: function


@pytest.fixture
def library(engine_library: str) -> Library:
    return Library.open(env={"XLSX_GRAFT_LIBRARY": engine_library})


def test_a_panic_status_returns_a_failure_carrying_the_panic_diagnostic(
    library: Library, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(library, "function", returning(2))
    result = version_call.call(library)
    assert result == Result(
        success=False,
        value=None,
        diagnostics=(
            Diagnostic("panic", "error", "the engine panicked; the call did not complete"),
        ),
    )
    assert result.failed


def test_a_null_arg_status_returns_a_failure_carrying_the_null_arg_diagnostic(
    library: Library, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(library, "function", returning(1))
    assert [d.code for d in version_call.call(library).diagnostics] == ["null_arg"]


def test_a_status_this_package_does_not_know_returns_a_failure_naming_it(
    library: Library, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(library, "function", returning(42))
    (diagnostic,) = version_call.call(library).diagnostics
    assert diagnostic.code == "unknown_status"
    assert "status 42" in diagnostic.message
