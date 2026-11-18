"""What :meth:`Viewer.tree <xlsx_graft.Viewer.tree>` shows: the parts of a
workbook and the relationships between them. Each class is one object of the
engine's JSON and each attribute one of its keys (``relationshipsKnown`` is
``relationships_known``). The names are made for display and are not exact.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class TreeTarget:
    """What a relationship points at: ``part``, the name of a part of the
    package, or ``external``, a target outside it. The other is ``None``.
    """

    part: str | None
    external: str | None


@dataclass(frozen=True)
class TreeRelationship:
    """One relationship: its ``id`` (``rId1``), its ``type`` (a URI) and its
    ``target``.
    """

    id: str
    type: str
    target: TreeTarget


@dataclass(frozen=True)
class TreePart:
    """One part of the package: its ``name`` (``/xl/workbook.xml``), its
    ``content_type`` (``None`` when the package gives none) and whether that
    is known, its ``size`` and ``compressed_size`` in bytes, whether it is
    ``readable``, whether another part ``referenced`` it (``None`` when that
    is not known), and its ``relationships`` with whether they are known.
    """

    name: str
    content_type: str | None
    content_type_known: bool
    size: int
    compressed_size: int
    readable: bool
    referenced: bool | None
    relationships_known: bool
    relationships: tuple[TreeRelationship, ...]


@dataclass(frozen=True)
class Tree:
    """The parts of a workbook (``complete`` is false when the listing
    stopped short), the relationships of the package itself and whether they
    are known.
    """

    complete: bool
    relationships_known: bool
    relationships: tuple[TreeRelationship, ...]
    parts: tuple[TreePart, ...]
