from __future__ import annotations

import pytest

from xlsx_graft.policy import policy_json, rules_json


class TestPolicyJson:
    def test_writes_the_keys_of_dicts_inside_dicts_and_lists_in_camel_case(self) -> None:
        policy = {
            "max_xml_depth": 512,
            "forbid": {
                "functions": [{"name": "HYPERLINK", "reason": "links"}],
                "parts": [{"kind": "external_link"}],
            },
        }
        assert policy_json(policy) == (
            '{"maxXmlDepth":512,"forbid":{"functions":[{"name":"HYPERLINK","reason":"links"}],'
            '"parts":[{"kind":"external_link"}]}}'
        )

    def test_leaves_a_camel_case_key_and_every_value_as_it_is(self) -> None:
        assert policy_json({"failOn": "warn", "max_entries": 7, "x": "a_b"}) == (
            '{"failOn":"warn","maxEntries":7,"x":"a_b"}'
        )

    def test_camelizes_the_keys_of_dicts_inside_a_list_but_not_inside_a_tuple(self) -> None:
        policy = {"a_list": [{"a_b": 1}, [{"c_d": 2}]], "a_tuple": ({"e_f": 3},)}
        assert policy_json(policy) == '{"aList":[{"aB":1},[{"cD":2}]],"aTuple":[{"e_f":3}]}'

    def test_writes_a_key_that_is_not_a_string_as_its_text(self) -> None:
        assert policy_json({1: "a"}) == '{"1":"a"}'

    def test_raises_type_error_for_a_policy_that_is_not_a_mapping(self) -> None:
        with pytest.raises(TypeError, match="not str"):
            policy_json("{}")

    def test_raises_type_error_for_a_value_json_cannot_write(self) -> None:
        with pytest.raises(TypeError):
            policy_json({"max_entries": object()})

    def test_raises_value_error_for_a_lone_surrogate_in_a_key_or_a_value(self) -> None:
        with pytest.raises(ValueError, match=r"lone surrogate \(U\+D800\), which UTF-8"):
            policy_json({"targets": ["\ud800"]})
        with pytest.raises(ValueError, match=r"lone surrogate \(U\+DC00\)"):
            policy_json({"a\udc00": 1})

    @pytest.mark.parametrize("value", [float("nan"), float("inf")])
    def test_raises_value_error_for_a_nan_or_an_infinity(self, value: float) -> None:
        with pytest.raises(ValueError, match="not JSON compliant"):
            policy_json({"max_entries": value})


class TestRulesJson:
    def test_raises_value_error_for_a_lone_surrogate_in_rules(self) -> None:
        with pytest.raises(ValueError, match=r"lone surrogate \(U\+D800\)"):
            rules_json({"targets": ["excel >= 2016\ud800"]})

    def test_writes_non_ascii_characters_as_ascii_escapes(self) -> None:
        assert rules_json({"a": "\U0001f600\u00e9"}) == '{"a":"\\ud83d\\ude00\\u00e9"}'

    def test_raises_value_error_for_a_surrogate_pair_written_as_two_code_units(self) -> None:
        with pytest.raises(ValueError, match=r"lone surrogate \(U\+D83D\)"):
            rules_json({"a": "\ud83d\ude00"})
