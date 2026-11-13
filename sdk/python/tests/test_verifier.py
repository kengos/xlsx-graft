from __future__ import annotations

import os
import threading
from pathlib import Path
from typing import Any

import pytest

import xlsx_graft
from xlsx_graft import Diagnostic, Error, PolicyInvalidError, Result, Verifier
from xlsx_graft.library import Library

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"
MACRO = FIXTURES / "macro.xlsm"
TABLE = FIXTURES / "table.xlsx"
DYNAMIC = FIXTURES / "dynamic-array.xlsx"


@pytest.fixture(scope="module")
def verifier(engine_library: str) -> Verifier:
    return Verifier(library=engine_library)


def codes(result: Result[None]) -> list[str]:
    return [d.code for d in result.diagnostics]


class TestVerify:
    def test_fails_a_workbook_the_engine_refuses_with_its_diagnostics_and_no_value(
        self, verifier: Verifier
    ) -> None:
        result = verifier.verify(MACRO)
        assert (result.success, result.error, result.value) == (False, None, None)
        assert codes(result) == ["part_denied", "part_denied"]
        first = result.diagnostics[0]
        assert first.severity == "error"
        assert first.decided_by is not None
        assert first.fix["in"] == "template"

    def test_lets_through_a_part_that_is_allowed_and_takes_a_path_of_each_kind(
        self, verifier: Verifier
    ) -> None:
        result = verifier.verify(MACRO, allow_parts=["macro"])
        assert (result.success, result.diagnostics) == (True, ())
        for path in (TABLE, str(TABLE), os.fsencode(TABLE)):
            assert verifier.verify(path) == Result(success=True, value=None)

    def test_fails_with_an_io_error_for_a_file_that_cannot_be_read(
        self, verifier: Verifier
    ) -> None:
        result = verifier.verify("/nowhere/book.xlsx")
        assert (result.success, result.diagnostics) == (False, ())
        assert result.error is not None
        assert result.error.code == "io"
        assert "/nowhere/book.xlsx" in result.error.message

    def test_reports_a_fault_of_the_rules_as_diagnostics_and_does_not_open_the_workbook(
        self, verifier: Verifier
    ) -> None:
        result = verifier.verify(
            "/nowhere/book.xlsx", targets=["excel 1990"], allow_functions=["_xlfn.SEQUENCE"]
        )
        assert (result.success, result.error) == (False, None)
        assert sorted(codes(result)) == ["rules_invalid", "target_invalid"]
        invalid = next(d for d in result.diagnostics if d.code == "rules_invalid")
        assert invalid.args["reason"] == "prefixed"

    def test_fails_on_a_warning_only_when_fail_on_says_so(self, verifier: Verifier) -> None:
        lenient = verifier.verify(DYNAMIC, best_effort=["excel 2016"])
        assert lenient.success
        assert "warn" in [d.severity for d in lenient.diagnostics]
        strict = verifier.verify(DYNAMIC, best_effort=["excel 2016"], fail_on="warn")
        assert (strict.success, strict.error) == (False, None)
        assert "error" not in [d.severity for d in strict.diagnostics]

    def test_accepts_every_rule_at_once_as_a_list_or_a_tuple(self, verifier: Verifier) -> None:
        result = verifier.verify(
            TABLE,
            targets=("excel >= 2016",),
            best_effort=["excel 2010"],
            allow_functions=["SEQUENCE"],
            allow_parts=["macro"],
            fail_on="error",
        )
        assert result.success

    @pytest.mark.parametrize("name", [b"\xff.xlsx", "\udcfe.xlsx"])
    def test_reads_a_file_whose_name_is_not_utf_8_and_verifies_its_bytes(
        self, verifier: Verifier, tmp_path: Path, name: str | bytes
    ) -> None:
        path = tmp_path / os.fsdecode(name)
        path.write_bytes(TABLE.read_bytes())
        assert verifier.verify(path).success
        assert verifier.verify(os.fsencode(path)).success

    def test_fails_with_an_io_error_for_a_file_that_is_not_utf_8_and_cannot_be_read(
        self, verifier: Verifier, tmp_path: Path
    ) -> None:
        result = verifier.verify(tmp_path / os.fsdecode(b"\xfd.xlsx"), targets=["excel 1990"])
        assert result.diagnostics == ()
        assert result.error is not None
        assert result.error.code == "io"
        assert result.error.message.startswith("the file could not be read: ")

    def test_raises_type_error_for_a_path_of_the_wrong_kind(self, verifier: Verifier) -> None:
        with pytest.raises(TypeError):
            verifier.verify(1)  # type: ignore[arg-type]

    def test_raises_type_error_for_a_rule_json_cannot_write(self, verifier: Verifier) -> None:
        with pytest.raises(TypeError):
            verifier.verify(TABLE, targets=[object()])  # type: ignore[list-item]

    def test_raises_value_error_for_a_lone_surrogate_in_a_rule(self, verifier: Verifier) -> None:
        with pytest.raises(ValueError, match="lone surrogate"):
            verifier.verify_bytes(b"", targets=["\ud800"])

    def test_raises_value_error_for_options_of_more_than_2_mib(self, verifier: Verifier) -> None:
        with pytest.raises(ValueError, match="the engine takes 2097152"):
            verifier.verify(TABLE, allow_functions=["A" * (2 << 20)])


class TestOptionsSize:
    def test_calls_the_engine_for_options_of_exactly_2_mib(self, verifier: Verifier) -> None:
        # {"policy":null,"rules":{"functions":{"allow":["<name>"]}}} around the name.
        overhead = len(b'{"policy":null,"rules":{"functions":{"allow":[""]}}}')
        name = "A" * ((2 << 20) - overhead)
        result = verifier.verify_bytes(b"", allow_functions=[name])
        assert result.error is None
        assert result.diagnostics
        with pytest.raises(ValueError, match="the engine takes 2097152"):
            verifier.verify_bytes(b"", allow_functions=[name + "A"])


class TestVerifyBytes:
    def test_verifies_the_bytes_and_fails_bytes_that_are_no_workbook_with_a_diagnostic_only(
        self, verifier: Verifier
    ) -> None:
        assert verifier.verify_bytes(TABLE.read_bytes()).success
        result = verifier.verify_bytes(b"")
        assert (result.success, result.error) == (False, None)
        assert result.diagnostics

    def test_takes_a_bytearray_and_a_memoryview(self, verifier: Verifier) -> None:
        data = bytearray(TABLE.read_bytes())
        assert verifier.verify_bytes(data).success
        assert verifier.verify_bytes(memoryview(data)).success

    def test_raises_type_error_for_what_is_not_bytes_like(self, verifier: Verifier) -> None:
        with pytest.raises(TypeError):
            verifier.verify_bytes("text")  # type: ignore[arg-type]

    def test_may_be_shared_by_threads_that_call_it_at_once(self, verifier: Verifier) -> None:
        data = MACRO.read_bytes()
        results: list[list[str]] = []

        def verify() -> None:
            for _ in range(5):
                results.append(codes(verifier.verify_bytes(data)))

        threads = [threading.Thread(target=verify) for _ in range(4)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
        assert results == [["part_denied", "part_denied"]] * 20


class TestPolicy:
    def test_writes_the_policy_to_the_engine_in_camel_case_so_its_max_entries_fails(
        self, engine_library: str
    ) -> None:
        result = Verifier({"max_entries": 1}, library=engine_library).verify(TABLE)
        assert result.success is False
        assert result.diagnostics[0].decided_by is not None
        assert result.diagnostics[0].decided_by["source"] == "policy"
        same = Verifier({"maxEntries": 1}, library=engine_library).verify(TABLE)
        assert codes(same) == codes(result)

    def test_lets_the_policys_fail_on_make_a_warning_fail(self, engine_library: str) -> None:
        verifier = Verifier({"fail_on": "warn"}, library=engine_library)
        assert verifier.verify(DYNAMIC, best_effort=["excel 2016"]).success is False

    def test_keeps_the_policy_it_checked_when_the_caller_changes_theirs(
        self, engine_library: str
    ) -> None:
        policy = {"max_entries": 1}
        verifier = Verifier(policy, library=engine_library)
        policy["max_entries"] = 10_000
        assert verifier.verify(TABLE).success is False

    def test_raises_policy_invalid_error_with_the_engines_diagnostics(
        self, engine_library: str
    ) -> None:
        policy = {"max_entries": 0, "forbid": {"parts": [{"kind": "nope"}]}}
        with pytest.raises(PolicyInvalidError) as raised:
            Verifier(policy, library=engine_library)
        diagnostics = raised.value.diagnostics
        assert [d.code for d in diagnostics] == ["policy_invalid"] * 2
        assert [d.args["key"] for d in diagnostics] == ["maxEntries", "forbid.parts[0]"]
        assert str(raised.value).startswith("the policy is not valid: ")
        assert isinstance(raised.value, Error)
        assert isinstance(raised.value, ValueError)

    def test_puts_the_error_before_the_diagnostics_when_the_engine_gives_one(
        self, engine_library: str, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        def policy(_self: Any, _policy: bytes) -> tuple[int, bytes]:
            error = '{"code":"io","message":"gone"}'
            record = (
                '{"code":"policy_invalid","severity":"error","message":"bad","args":{},'
                '"location":{},"decidedBy":null,"fix":null}'
            )
            return 0, f'{{"ok":false,"error":{error},"diagnostics":[{record}]}}'.encode()

        monkeypatch.setattr("xlsx_graft.verify_call.VerifyCall.check_policy", policy)
        with pytest.raises(PolicyInvalidError) as raised:
            Verifier({}, library=engine_library)
        assert raised.value.diagnostics == (
            Diagnostic("io", "error", "gone"),
            Diagnostic("policy_invalid", "error", "bad", {}, {}),
        )
        assert str(raised.value) == "the policy is not valid: gone; bad"

    def test_raises_type_error_for_a_policy_that_is_not_a_mapping(
        self, engine_library: str
    ) -> None:
        with pytest.raises(TypeError):
            Verifier("{}", library=engine_library)  # type: ignore[arg-type]

    def test_raises_value_error_for_a_nan_in_the_policy(self, engine_library: str) -> None:
        with pytest.raises(ValueError, match="not JSON compliant"):
            Verifier({"max_entries": float("nan")}, library=engine_library)

    def test_raises_value_error_before_the_engine_checks_a_policy_with_a_lone_surrogate(
        self, engine_library: str
    ) -> None:
        policy = {"forbid": {"functions": [{"name": "\ud800"}]}}
        with pytest.raises(ValueError, match="lone surrogate") as raised:
            Verifier(policy, library=engine_library)
        assert not isinstance(raised.value, PolicyInvalidError)


def test_passes_library_on_to_library_open(
    monkeypatch: pytest.MonkeyPatch, engine_library: str
) -> None:
    paths: list[str | None] = []
    original = Library.open

    def spy(*, path: str | None = None) -> Library:
        paths.append(path)
        return original(path=engine_library)

    monkeypatch.setattr(Library, "open", spy)
    Verifier(library="/elsewhere.so")
    assert paths == ["/elsewhere.so"]


def test_is_exported_from_the_package() -> None:
    assert xlsx_graft.Verifier is Verifier
    assert xlsx_graft.PolicyInvalidError is PolicyInvalidError
