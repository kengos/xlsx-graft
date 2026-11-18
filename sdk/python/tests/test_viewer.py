from __future__ import annotations

import os
import threading
from pathlib import Path
from typing import Any

import pytest

import xlsx_graft
from xlsx_graft import Grid, GridCell, GridRow, GridTruncated, Tree, Viewer
from xlsx_graft.library import Library
from xlsx_graft.row import SheetInfo

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"
TABLE = FIXTURES / "table.xlsx"
MERGED = FIXTURES / "merged.xlsx"
EXTERNAL = FIXTURES / "external-link.xlsx"


@pytest.fixture(scope="module")
def viewer(engine_library: str) -> Viewer:
    return Viewer(library=engine_library)


def cell(grid: Grid, ref: str) -> GridCell:
    return next(c for row in grid.rows for c in row.cells if c.ref == ref)


def grid_of(viewer: Viewer, **keywords: Any) -> Grid:
    grid = viewer.describe_sheet(MERGED, **keywords).value
    assert grid is not None
    return grid


class TestTree:
    def test_lists_the_parts_and_the_relationships_of_the_workbook_at_a_path(
        self, viewer: Viewer
    ) -> None:
        result = viewer.tree(TABLE)
        tree = result.value
        assert (result.success, result.error) == (True, None)
        assert isinstance(tree, Tree)
        assert tree.complete is True
        assert "/xl/workbook.xml" in [part.name for part in tree.parts]
        assert "/xl/workbook.xml" in [r.target.part for r in tree.relationships]

    def test_gives_the_same_tree_for_a_path_of_each_kind_and_for_the_bytes_of_the_file(
        self, viewer: Viewer
    ) -> None:
        tree = viewer.tree(TABLE).value
        assert viewer.tree(str(TABLE)).value == tree
        assert viewer.tree(os.fsencode(TABLE)).value == tree
        assert viewer.tree_bytes(TABLE.read_bytes()).value == tree
        assert viewer.tree_bytes(bytearray(TABLE.read_bytes())).value == tree
        assert viewer.tree_bytes(memoryview(TABLE.read_bytes())).value == tree

    def test_gives_a_target_outside_the_package_as_external_with_no_part(
        self, viewer: Viewer
    ) -> None:
        tree = viewer.tree(EXTERNAL).value
        assert tree is not None
        targets = [r.target for part in tree.parts for r in part.relationships]
        external = next(t for t in targets if t.external is not None)
        assert external.part is None
        assert isinstance(external.external, str)

    def test_fails_with_an_io_error_and_a_tree_with_no_parts_for_a_file_that_does_not_exist(
        self, viewer: Viewer
    ) -> None:
        result = viewer.tree("/nowhere/book.xlsx")
        assert result.success is False
        assert result.error is not None
        assert result.error.code == "io"
        assert isinstance(result.value, Tree)
        assert result.value.parts == ()

    def test_fails_with_diagnostics_and_a_tree_with_no_parts_for_bytes_that_are_not_a_workbook(
        self, viewer: Viewer
    ) -> None:
        result = viewer.tree_bytes(b"not a zip")
        assert (result.success, result.error) == (False, None)
        assert result.diagnostics
        assert isinstance(result.value, Tree)
        assert result.value.parts == ()

    @pytest.mark.parametrize("name", [b"\xff.xlsx", "\udcfe.xlsx"])
    def test_reads_a_file_whose_name_is_not_utf_8_and_gives_the_same_tree(
        self, viewer: Viewer, tmp_path: Path, name: str | bytes
    ) -> None:
        path = tmp_path / os.fsdecode(name)
        path.write_bytes(TABLE.read_bytes())
        assert viewer.tree(path).value == viewer.tree(TABLE).value

    def test_has_no_value_for_a_file_whose_name_is_not_utf_8_and_that_cannot_be_read(
        self, viewer: Viewer, tmp_path: Path
    ) -> None:
        result = viewer.tree(tmp_path / os.fsdecode(b"\xfe.xlsx"))
        assert result.error is not None
        assert result.error.code == "io"
        assert "could not be read" in result.error.message
        assert result.value is None

    def test_raises_type_error_for_a_path_of_the_wrong_kind_and_for_what_is_not_bytes_like(
        self, viewer: Viewer
    ) -> None:
        with pytest.raises(TypeError):
            viewer.tree(1)  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            viewer.tree_bytes("text")  # type: ignore[arg-type]


class TestDescribeSheet:
    def test_shows_the_sheet_the_ranges_and_the_merges_of_the_first_sheet(
        self, viewer: Viewer
    ) -> None:
        result = viewer.describe_sheet(MERGED)
        assert (result.success, result.error) == (True, None)
        grid = result.value
        assert grid is not None
        assert grid.sheet == SheetInfo(index=0, name="Sheet1", kind="worksheet", state="visible")
        assert (grid.dimension, grid.used, grid.shown) == ("A1:E8", "A1:E8", "A1:E8")
        assert grid.complete is True
        assert grid.truncated == GridTruncated(rows=False, columns=False, merges=False)
        assert grid.merges == ("A1:C1", "A3:A5")

    def test_shows_the_cells_of_the_first_sheet(self, viewer: Viewer) -> None:
        grid = grid_of(viewer)
        assert grid.rows[0] == GridRow(
            row=1,
            cells=(GridCell(ref="A1", type="string", value="Quarterly report", truncated=False),),
        )
        assert (cell(grid, "C3").type, cell(grid, "C3").value) == ("number", 1.5)
        assert isinstance(cell(grid, "C4").value, float)
        assert cell(grid, "C4").value == 2.0
        assert (cell(grid, "C6").type, cell(grid, "C6").value) == ("date", "2024-01-15")

    def test_gives_the_same_grid_for_a_path_of_each_kind_and_for_the_bytes_of_the_file(
        self, viewer: Viewer
    ) -> None:
        grid = grid_of(viewer)
        assert viewer.describe_sheet(str(MERGED)).value == grid
        assert viewer.describe_sheet(os.fsencode(MERGED)).value == grid
        assert viewer.describe_sheet_bytes(MERGED.read_bytes()).value == grid

    def test_takes_the_window_and_the_sheet_the_keywords_name(self, viewer: Viewer) -> None:
        assert grid_of(viewer, range="A1:C3").shown == "A1:C3"
        assert grid_of(viewer, max_rows=2).shown == "A1:E2"
        assert grid_of(viewer, max_columns=2).shown == "A1:B8"
        assert grid_of(viewer, sheet="Sheet1").sheet is not None
        by_bytes = viewer.describe_sheet_bytes(
            MERGED.read_bytes(), sheet="Sheet1", range="A1:B2", max_rows=2, max_columns=1
        )
        assert by_bytes.value is not None
        assert by_bytes.value.shown == "A1:A2"

    def test_takes_the_index_of_the_sheet_as_an_int(self, viewer: Viewer) -> None:
        assert grid_of(viewer, sheet=0) == grid_of(viewer)
        by_bytes = viewer.describe_sheet_bytes(MERGED.read_bytes(), sheet=0)
        assert by_bytes.value == grid_of(viewer)

    def test_reads_a_file_whose_name_is_not_utf_8_as_bytes_and_shows_the_same_grid(
        self, viewer: Viewer, tmp_path: Path
    ) -> None:
        path = tmp_path / os.fsdecode(b"\xff.xlsx")
        path.write_bytes(MERGED.read_bytes())
        assert viewer.describe_sheet(path).value == grid_of(viewer)

    def test_fails_with_sheet_not_found_and_a_grid_whose_sheet_is_none(
        self, viewer: Viewer
    ) -> None:
        result = viewer.describe_sheet(TABLE, sheet="Nope")
        assert result.success is False
        assert result.error is not None
        assert result.error.code == "sheet_not_found"
        assert isinstance(result.value, Grid)
        assert (result.value.sheet, result.value.rows) == (None, ())

    def test_has_no_value_for_a_file_whose_name_is_not_utf_8_and_that_cannot_be_read(
        self, viewer: Viewer, tmp_path: Path
    ) -> None:
        result = viewer.describe_sheet(tmp_path / os.fsdecode(b"\xfe.xlsx"))
        assert result.error is not None
        assert result.error.code == "io"
        assert result.value is None

    @pytest.mark.parametrize(
        ("keywords", "message"),
        [
            ({"range": "nope"}, "range"),
            ({"max_rows": 0}, "maxRows"),
            ({"sheet": 1.5}, "sheet"),
            ({"sheet": True}, "sheet"),
        ],
    )
    def test_raises_value_error_with_the_engines_message_for_a_value_the_engine_refuses(
        self, viewer: Viewer, keywords: dict[str, Any], message: str
    ) -> None:
        with pytest.raises(ValueError, match=message):
            viewer.describe_sheet(MERGED, **keywords)
        with pytest.raises(ValueError, match=message):
            viewer.describe_sheet_bytes(MERGED.read_bytes(), **keywords)

    @pytest.mark.parametrize(
        ("keywords", "message"),
        [
            ({"max_rows": 10**400}, "max_rows"),
            ({"range": "\ud800"}, "lone surrogate"),
            ({"range": "A" * (2 << 20)}, "the engine takes 2097152"),
            ({"max_columns": float("nan")}, "not JSON compliant"),
        ],
    )
    def test_raises_value_error_before_the_call_for_options_that_cannot_be_sent(
        self, viewer: Viewer, keywords: dict[str, Any], message: str
    ) -> None:
        with pytest.raises(ValueError, match=message):
            viewer.describe_sheet("/nowhere", **keywords)
        with pytest.raises(ValueError, match=message):
            viewer.describe_sheet_bytes(b"", **keywords)

    def test_raises_value_error_for_options_before_it_reads_a_path_that_is_not_utf_8(
        self, viewer: Viewer, tmp_path: Path
    ) -> None:
        with pytest.raises(ValueError, match="max_rows"):
            viewer.describe_sheet(tmp_path / os.fsdecode(b"\xfe.xlsx"), max_rows=10**400)

    def test_raises_type_error_for_a_value_json_cannot_write(self, viewer: Viewer) -> None:
        with pytest.raises(TypeError):
            viewer.describe_sheet(MERGED, range=object())  # type: ignore[arg-type]


def test_may_be_shared_by_threads_that_call_it_at_once(viewer: Viewer) -> None:
    data = MERGED.read_bytes()
    grids: list[Grid | None] = []

    def describe() -> None:
        for _ in range(5):
            grids.append(viewer.describe_sheet_bytes(data).value)

    threads = [threading.Thread(target=describe) for _ in range(4)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    assert len(grids) == 20
    assert all(grid == grids[0] for grid in grids)
    assert grids[0] is not None
    assert grids[0].rows


def test_passes_library_on_to_library_open(
    monkeypatch: pytest.MonkeyPatch, engine_library: str
) -> None:
    paths: list[str | None] = []
    original = Library.open

    def spy(*, path: str | None = None) -> Library:
        paths.append(path)
        return original(path=engine_library)

    monkeypatch.setattr(Library, "open", spy)
    Viewer(library="/elsewhere.so")
    assert paths == ["/elsewhere.so"]


def test_is_exported_from_the_package() -> None:
    assert xlsx_graft.Viewer is Viewer
    assert xlsx_graft.Tree is Tree
    assert xlsx_graft.Grid is Grid
    names = {"TreePart", "TreeRelationship", "TreeTarget", "GridTruncated", "GridRow", "GridCell"}
    assert names <= set(xlsx_graft.__all__)
