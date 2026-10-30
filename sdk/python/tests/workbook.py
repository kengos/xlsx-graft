"""Workbooks for the tests to read, built as a test asks: a zip of stored
entries holding a package with a shared strings table of "a" and "b", styles
whose cell formats 1 to 4 are a date, a time, a date-time and an elapsed
time, and the sheets the test gives. The same workbook as
engine/capi/src/tests/workbook.rs builds, written again in Python.
"""

from __future__ import annotations

import struct
import zlib
from dataclasses import dataclass

NS = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
TYPES = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
RELS = "http://schemas.openxmlformats.org/package/2006/relationships"
CHARTSHEET = f"{TYPES}/chartsheet"
WORKSHEET = f"{TYPES}/worksheet"
CONTENT_TYPES = (
    '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
    '<Default Extension="rels" '
    'ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
    '<Default Extension="xml" ContentType="application/xml"/></Types>'
)


@dataclass(frozen=True)
class Sheet:
    """A sheet: its name, its ``state`` (None: none written), the type of its
    relationship (None: its r:id names none) and its part's XML.
    """

    name: str
    state: str | None
    rel_type: str | None
    xml: str


def worksheet(name: str, rows: str = "", state: str | None = None) -> Sheet:
    """A worksheet named ``name`` whose sheetData holds ``rows``."""
    xml = f'<worksheet xmlns="{NS}"><sheetData>{rows}</sheetData></worksheet>'
    return Sheet(name, state, WORKSHEET, xml)


def chartsheet(name: str) -> Sheet:
    return Sheet(name, None, CHARTSHEET, f'<chartsheet xmlns="{NS}"/>')


def unrelated(name: str) -> Sheet:
    """A sheet named ``name`` whose r:id names no relationship."""
    return Sheet(name, None, None, "")


def xlsx(*sheets: Sheet) -> bytes:
    """The bytes of a workbook of ``sheets``, in order, each at
    xl/sheets/sheet<n>.xml.
    """
    numbered = list(enumerate(sheets, 1))
    entries = {
        "[Content_Types].xml": CONTENT_TYPES,
        "_rels/.rels": _rels(_rel("rId1", f"{TYPES}/officeDocument", "xl/workbook.xml")),
        "xl/workbook.xml": _workbook(numbered),
        "xl/_rels/workbook.xml.rels": _rels(_related(numbered)),
        "xl/sst.xml": f'<sst xmlns="{NS}"><si><t>a</t></si><si><t>b</t></si></sst>',
        "xl/styles.xml": _styles(),
    }
    for n, sheet in numbered:
        if sheet.rel_type:
            entries[f"xl/sheets/sheet{n}.xml"] = sheet.xml
    return _zip(entries)


def _workbook(numbered: list[tuple[int, Sheet]]) -> str:
    listed = "".join(
        f'<sheet name="{sheet.name}" sheetId="{n}"'
        + (f' state="{sheet.state}"' if sheet.state else "")
        + f' r:id="rId{n}"/>'
        for n, sheet in numbered
    )
    return f'<workbook xmlns="{NS}" xmlns:r="{TYPES}"><sheets>{listed}</sheets></workbook>'


def _related(numbered: list[tuple[int, Sheet]]) -> str:
    sheets = "".join(
        _rel(f"rId{n}", sheet.rel_type, f"sheets/sheet{n}.xml")
        for n, sheet in numbered
        if sheet.rel_type
    )
    return (
        _rel("rIdS", f"{TYPES}/sharedStrings", "sst.xml")
        + _rel("rIdT", f"{TYPES}/styles", "styles.xml")
        + sheets
    )


def _styles() -> str:
    formats = "".join(f'<xf numFmtId="{id_}"/>' for id_ in (0, 14, 21, 22, 46))
    return f'<styleSheet xmlns="{NS}"><cellXfs>{formats}</cellXfs></styleSheet>'


def _rel(id_: str, type_: str, target: str) -> str:
    return f'<Relationship Id="{id_}" Type="{type_}" Target="{target}"/>'


def _rels(body: str) -> str:
    return f'<Relationships xmlns="{RELS}">{body}</Relationships>'


def _zip(entries: dict[str, str]) -> bytes:
    """The zip of ``entries``, each stored, in order."""
    out = b""
    directory = b""
    for name, text in entries.items():
        data = text.encode()
        offset = len(out)
        size = len(data)
        header = (20, 0, 0, 0, 0, zlib.crc32(data), size, size, len(name))
        out += struct.pack("<IHHHHHIIIHH", 0x04034B50, *header, 0) + name.encode() + data
        central = (0x02014B50, 20, *header, 0, 0, 0, 0, 0, offset)
        directory += struct.pack("<IHHHHHHIIIHHHHHII", *central)
        directory += name.encode()
    count = len(entries)
    trailer = struct.pack("<IHHHHIIH", 0x06054B50, 0, 0, count, count, len(directory), len(out), 0)
    return out + directory + trailer
