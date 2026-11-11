from __future__ import annotations

import pytest

from xlsx_graft import Error, verify_report

RECORD = (
    '{"code":"part_denied","severity":"error","message":"m","args":{},"location":{},'
    '"decidedBy":null,"fix":null,"origin":"x"}'
)


def report(*, ok: str, error: str = "null", records: str = "") -> str:
    return f'{{"ok":{ok},"failOn":"error","counts":{{}},"error":{error},"diagnostics":[{records}]}}'


def test_is_a_success_with_no_value_when_ok_whatever_diagnostics_it_has() -> None:
    result = verify_report.call(0, report(ok="true", records=RECORD).encode())
    assert (result.success, result.value, result.error) == (True, None, None)
    assert [d.code for d in result.diagnostics] == ["part_denied"]


def test_fails_with_the_engines_diagnostics_and_the_results_io_error_as_the_error() -> None:
    result = verify_report.call(0, report(ok="false", records=RECORD).encode())
    assert (result.success, result.value, result.error) == (False, None, None)
    assert [d.code for d in result.diagnostics] == ["part_denied"]
    io = verify_report.call(0, report(ok="false", error='{"code":"io","message":"gone"}').encode())
    assert io.error is not None
    assert (io.error.code, io.error.message, io.error.severity) == ("io", "gone", "error")


@pytest.mark.parametrize(
    ("status", "code"), [(1, "null_arg"), (2, "panic"), (99, "unknown_status")]
)
def test_fails_with_an_error_naming_a_status_other_than_ok_and_invalid_arg(
    status: int, code: str
) -> None:
    result = verify_report.call(status, b"")
    assert (result.success, result.diagnostics) == (False, ())
    assert result.error is not None
    assert result.error.code == code


def test_raises_error_for_invalid_arg_which_only_a_call_built_wrongly_gets() -> None:
    with pytest.raises(Error, match="a bug of the package"):
        verify_report.call(4, b"")


@pytest.mark.parametrize(
    "text",
    [
        "not json",
        "",
        "{}",
        "[]",
        "null",
        report(ok="true", records="1"),
        report(ok="false", error='"io"'),
        report(ok="false", error="{}"),
        '{"ok":true,"error":null,"diagnostics":5}',
        '{"ok":true,"error":null,"diagnostics":""}',
        '{"ok":true,"error":null,"diagnostics":{}}',
        '{"ok":1,"error":null,"diagnostics":[]}',
        '{"ok":"true","error":null,"diagnostics":[]}',
        '{"ok":null,"error":null,"diagnostics":[]}',
    ],
)
def test_fails_with_diagnostics_unreadable_for_json_that_is_not_a_result(text: str) -> None:
    result = verify_report.call(0, text.encode())
    assert result.success is False
    assert result.error is not None
    assert result.error.code == "diagnostics_unreadable"
    assert result.error.message.startswith("the engine's result could not be read: ")


def test_fails_with_diagnostics_unreadable_for_a_result_that_is_not_utf_8() -> None:
    result = verify_report.call(0, b'{"ok":true,"diagnostics":[],"x":"\xff"}')
    assert result.success is False
    assert result.error is not None
    assert result.error.code == "diagnostics_unreadable"


def test_does_not_read_the_error_of_a_result_that_is_ok() -> None:
    result = verify_report.call(0, report(ok="true", error='"not an object"').encode())
    assert (result.success, result.error) == (True, None)
