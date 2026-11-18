from __future__ import annotations

import pytest

from xlsx_graft import (
    Error,
    Grid,
    GridCell,
    GridTruncated,
    Result,
    Tree,
    TreeRelationship,
    TreeTarget,
    view_content,
    view_report,
)
from xlsx_graft.row import SheetInfo

TREE = (
    '{"ok":true,"error":null,"diagnostics":[],"complete":false,"relationshipsKnown":true,'
    '"relationships":[{"id":"rId1","type":"t","target":{"part":"/a.xml"}}],'
    '"parts":[{"name":"/a.xml","contentType":null,"contentTypeKnown":false,"size":3,'
    '"compressedSize":2,"readable":true,"referenced":null,"relationshipsKnown":false,'
    '"relationships":[{"id":"rId2","type":"x","target":{"external":"http://e/"}}],'
    '"later":1}],"laterKey":[]}'
)
SHEET = '{"index":1,"name":"S","kind":"worksheet","state":"hidden","later":2}'
RECORD = (
    '{"code":"part_denied","severity":"error","message":"m","args":{},"location":{},'
    '"decidedBy":null,"fix":null,"origin":"x"}'
)


def grid(cells: str, *, sheet: str = SHEET, ok: str = "true", error: str = "null") -> str:
    return (
        f'{{"ok":{ok},"error":{error},"diagnostics":[{RECORD}],"sheet":{sheet},'
        '"dimension":null,"used":"A1:A1","shown":"A1:A1",'
        '"truncated":{"rows":false,"columns":true,"merges":false},"complete":true,'
        f'"rows":[{{"row":1,"cells":[{cells}]}}],"merges":["A1:B2"]}}'
    )


def read_grid(text: str) -> Result[Grid]:
    return view_report.call(0, text.encode(), view_content.grid)


def test_reads_a_tree_ignoring_keys_it_does_not_know() -> None:
    result = view_report.call(0, TREE.encode(), view_content.tree)
    assert (result.success, result.error, result.diagnostics) == (True, None, ())
    tree = result.value
    assert isinstance(tree, Tree)
    assert (tree.complete, tree.relationships_known) == (False, True)
    assert tree.relationships == (
        TreeRelationship("rId1", "t", TreeTarget(part="/a.xml", external=None)),
    )
    part = tree.parts[0]
    assert (part.name, part.content_type, part.content_type_known) == ("/a.xml", None, False)
    assert (part.size, part.compressed_size, part.readable, part.referenced) == (3, 2, True, None)
    assert part.relationships_known is False
    assert part.relationships[0].target == TreeTarget(part=None, external="http://e/")


def test_reads_each_type_of_cell_as_the_json_gives_it_and_a_cell_without_truncated_is_whole() -> (
    None
):
    cells = ",".join(
        [
            '{"ref":"A1","type":"number","value":2.0}',
            '{"ref":"B1","type":"elapsed","value":90000}',
            '{"ref":"C1","type":"boolean","value":true}',
            '{"ref":"D1","type":"error","value":"#N/A"}',
            '{"ref":"E1","type":"datetime","value":"2024-01-15T08:30:00.000"}',
            '{"ref":"F1","type":"string","value":"cut","truncated":true,"later":1}',
        ]
    )
    value = read_grid(grid(cells)).value
    assert value is not None
    got = [(c.value, c.truncated) for c in value.rows[0].cells]
    assert got == [
        (2.0, False),
        (90000, False),
        (True, False),
        ("#N/A", False),
        ("2024-01-15T08:30:00.000", False),
        ("cut", True),
    ]
    assert isinstance(value.rows[0].cells[0].value, float)
    assert isinstance(value.rows[0].cells[1].value, int)
    assert value.rows[0].cells[0] == GridCell("A1", "number", 2.0, truncated=False)


def test_reads_the_sheet_the_flags_and_the_merges_of_a_grid() -> None:
    result = read_grid(grid(""))
    assert result.success is True
    assert [d.code for d in result.diagnostics] == ["part_denied"]
    assert result.value is not None
    assert result.value.sheet == SheetInfo(index=1, name="S", kind="worksheet", state="hidden")
    assert result.value.truncated == GridTruncated(rows=False, columns=True, merges=False)
    assert (result.value.dimension, result.value.used, result.value.shown) == (
        None,
        "A1:A1",
        "A1:A1",
    )
    assert (result.value.complete, result.value.merges) == (True, ("A1:B2",))
    assert result.value.rows[0].row == 1


def test_has_no_sheet_for_a_null_sheet_and_keeps_the_value_of_a_failure() -> None:
    error = '{"code":"io","message":"gone"}'
    result = read_grid(grid("", sheet="null", ok="false", error=error))
    assert (result.success, result.diagnostics[0].code) == (False, "part_denied")
    assert result.error is not None
    assert (result.error.code, result.error.message) == ("io", "gone")
    assert isinstance(result.value, Grid)
    assert result.value.sheet is None


def test_keeps_the_value_of_a_failure_that_has_no_error() -> None:
    result = read_grid(grid("", ok="false"))
    assert (result.success, result.error) == (False, None)
    assert isinstance(result.value, Grid)


def test_raises_value_error_for_request_invalid_with_the_message() -> None:
    with pytest.raises(ValueError, match=r"^maxRows 0 is not 1 to 10000$"):
        view_report.call(11, b"maxRows 0 is not 1 to 10000", view_content.grid)


def test_raises_value_error_for_request_invalid_with_text_that_is_not_utf_8() -> None:
    with pytest.raises(ValueError, match=r"^bad \ufffd$"):
        view_report.call(11, b"bad \xff", view_content.grid)


def test_raises_value_error_with_a_fixed_message_for_request_invalid_with_no_message() -> None:
    with pytest.raises(ValueError, match=r"^the engine refused a value of the options") as raised:
        view_report.call(11, b"", view_content.grid)
    assert str(raised.value).endswith("(status 11) and gave no reason")


def test_raises_error_for_invalid_arg_which_only_a_call_built_wrongly_gets() -> None:
    with pytest.raises(Error, match=r"status 4.*a bug of the package"):
        view_report.call(4, b"", view_content.grid)


@pytest.mark.parametrize(
    ("status", "code"), [(1, "null_arg"), (2, "panic"), (99, "unknown_status")]
)
def test_fails_with_no_value_for_a_status_other_than_ok_request_invalid_and_invalid_arg(
    status: int, code: str
) -> None:
    result = view_report.call(status, b"", view_content.grid)
    assert (result.success, result.diagnostics, result.value) == (False, (), None)
    assert result.error is not None
    assert result.error.code == code


@pytest.mark.parametrize(
    "text",
    [
        "not json",
        "",
        "{}",
        '{"ok":true}',
        "[]",
        "null",
        '"x"',
        grid("").replace('"merges":["A1:B2"]', '"merges":5'),
        grid("", sheet="false"),
        grid("", sheet='"S"'),
        grid('{"ref":"A1","value":1}'),
        grid("1"),
        grid("", ok="false", error='"io"'),
        grid("", ok="false", error="{}"),
    ],
)
def test_fails_with_diagnostics_unreadable_and_no_value_for_json_that_is_not_a_view(
    text: str,
) -> None:
    result = read_grid(text)
    assert (result.success, result.diagnostics, result.value) == (False, (), None)
    assert result.error is not None
    assert result.error.code == "diagnostics_unreadable"
    assert result.error.message.startswith("the engine's result could not be read: ")


def test_fails_with_diagnostics_unreadable_for_a_relationship_target_that_is_not_an_object() -> (
    None
):
    text = TREE.replace('"target":{"part":"/a.xml"}', '"target":"/a.xml"')
    result = view_report.call(0, text.encode(), view_content.tree)
    assert result.error is not None
    assert (result.error.code, result.value) == ("diagnostics_unreadable", None)


def test_fails_with_diagnostics_unreadable_for_a_result_that_is_not_utf_8() -> None:
    result = view_report.call(0, b'{"ok":true,"x":"\xff"}', view_content.tree)
    assert result.error is not None
    assert result.error.code == "diagnostics_unreadable"
