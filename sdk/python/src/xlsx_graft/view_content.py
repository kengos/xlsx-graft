"""Reads the keys of the ``tree`` and ``describe_sheet`` JSON after ``ok``,
``error`` and ``diagnostics`` into a :class:`~xlsx_graft.Tree` or a
:class:`~xlsx_graft.Grid`. A key it does not know is ignored, and the type of
a value is not checked: JSON that lacks a key, or whose value has the wrong
type for what is done with it, raises ``KeyError``, ``TypeError`` or
``AttributeError``.
"""

from __future__ import annotations

from typing import Any

from xlsx_graft.grid import Grid, GridCell, GridRow, GridTruncated
from xlsx_graft.row import SheetInfo
from xlsx_graft.tree import Tree, TreePart, TreeRelationship, TreeTarget


def tree(view: dict[str, Any]) -> Tree:
    """The tree the parsed JSON ``view`` holds."""
    return Tree(
        complete=view["complete"],
        relationships_known=view["relationshipsKnown"],
        relationships=_relationships(view["relationships"]),
        parts=tuple(_part(part) for part in view["parts"]),
    )


def grid(view: dict[str, Any]) -> Grid:
    """The grid the parsed JSON ``view`` holds."""
    return Grid(
        sheet=_sheet(view["sheet"]),
        dimension=view["dimension"],
        used=view["used"],
        shown=view["shown"],
        truncated=_truncated(view["truncated"]),
        complete=view["complete"],
        rows=tuple(_row(row) for row in view["rows"]),
        merges=tuple(view["merges"]),
    )


def _part(part: dict[str, Any]) -> TreePart:
    return TreePart(
        name=part["name"],
        content_type=part["contentType"],
        content_type_known=part["contentTypeKnown"],
        size=part["size"],
        compressed_size=part["compressedSize"],
        readable=part["readable"],
        referenced=part["referenced"],
        relationships_known=part["relationshipsKnown"],
        relationships=_relationships(part["relationships"]),
    )


def _relationships(records: list[dict[str, Any]]) -> tuple[TreeRelationship, ...]:
    return tuple(
        TreeRelationship(
            id=record["id"],
            type=record["type"],
            target=TreeTarget(
                part=record["target"].get("part"), external=record["target"].get("external")
            ),
        )
        for record in records
    )


def _sheet(sheet: dict[str, Any] | None) -> SheetInfo | None:
    if sheet is None:
        return None
    return SheetInfo(
        index=sheet["index"], name=sheet["name"], kind=sheet["kind"], state=sheet["state"]
    )


def _truncated(truncated: dict[str, Any]) -> GridTruncated:
    return GridTruncated(
        rows=truncated["rows"], columns=truncated["columns"], merges=truncated["merges"]
    )


def _row(row: dict[str, Any]) -> GridRow:
    return GridRow(row=row["row"], cells=tuple(_cell(cell) for cell in row["cells"]))


def _cell(cell: dict[str, Any]) -> GridCell:
    return GridCell(
        ref=cell["ref"],
        type=cell["type"],
        value=cell["value"],
        truncated=cell.get("truncated", False),
    )
