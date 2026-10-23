"""The package downloads nothing and depends on nothing (docs/sdk.md § Finding
the engine library): ctypes is the standard library.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
SOURCES = sorted((PACKAGE / "src" / "xlsx_graft").rglob("*.py"))

NETWORKING = re.compile(
    r"^\s*(?:import|from)\s+"
    r"(socket|ssl|urllib|http|ftplib|smtplib|asyncio|requests|httpx|aiohttp|urllib3)\b",
    re.MULTILINE,
)


def test_reads_every_source_file() -> None:
    # A sweep over no files reports no offenders and proves nothing.
    assert {path.name for path in SOURCES} >= {"__init__.py", "library.py", "version_call.py"}


def test_imports_no_networking_library() -> None:
    offenders = [path.name for path in SOURCES if NETWORKING.search(path.read_text("utf-8"))]
    assert offenders == []


def test_the_sweep_matches_a_networking_import() -> None:
    assert NETWORKING.search("import socket\n")
    assert NETWORKING.search("from urllib.request import urlopen\n")


def test_declares_no_runtime_dependencies() -> None:
    manifest = tomllib.loads((PACKAGE / "pyproject.toml").read_text(encoding="utf-8"))
    assert manifest["project"]["dependencies"] == []
