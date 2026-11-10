"""Checks the nuspec of each nupkg the way nuget.org reads it.

    python3 -I scripts/release/dry-run/check_nuspec.py <repository> <nupkg>...

Every nupkg names an id, a version (sdk/dotnet/XlsxGraft/XlsxGraft.csproj's),
authors and a description; the license as an SPDX expression; the repository;
and a readme that is in the package. `dotnet pack` also writes a `licenseUrl`
(a link to the license), which this script does not check. Every failure is
printed, then the exit status is 1.
"""

from __future__ import annotations

import re
import sys
import zipfile
from pathlib import Path
from xml.etree import ElementTree

LICENSE = "MIT OR Apache-2.0 OR BSD-3-Clause"


def text(metadata: ElementTree.Element, tag: str) -> str:
    """The text of a child of <metadata>, whatever the nuspec's namespace."""
    for child in metadata:
        if child.tag.rsplit("}", 1)[-1] == tag:
            return (child.text or "").strip()
    return ""


def attribute(metadata: ElementTree.Element, tag: str, name: str) -> str:
    for child in metadata:
        if child.tag.rsplit("}", 1)[-1] == tag:
            return child.get(name, "")
    return ""


def check(path: Path, version: str) -> list[str]:
    with zipfile.ZipFile(path) as archive:
        nuspecs = [n for n in archive.namelist() if n.endswith(".nuspec") and "/" not in n]
        if len(nuspecs) != 1:
            return [f"{path.name}: {len(nuspecs)} nuspec files, not 1"]
        metadata = ElementTree.fromstring(archive.read(nuspecs[0]))[0]
        names = set(archive.namelist())
    problems = []
    for field in ("id", "authors", "description", "readme"):
        if not text(metadata, field):
            problems.append(f"{path.name}: no <{field}>")
    if text(metadata, "version") != version:
        problems.append(f"{path.name}: version {text(metadata, 'version')}, not {version}")
    if attribute(metadata, "license", "type") != "expression" or text(metadata, "license") != LICENSE:
        problems.append(f"{path.name}: <license> is not the expression {LICENSE}")
    if not attribute(metadata, "repository", "url"):
        problems.append(f"{path.name}: no <repository url>")
    if text(metadata, "readme") and text(metadata, "readme") not in names:
        problems.append(f"{path.name}: the readme {text(metadata, 'readme')} is not in the package")
    return problems


def main(root: str, *nupkgs: str) -> int:
    csproj = (Path(root) / "sdk/dotnet/XlsxGraft/XlsxGraft.csproj").read_text(encoding="utf-8")
    found = re.search(r"<Version>([^<]+)</Version>", csproj)
    if found is None:
        sys.exit("check_nuspec: no <Version> in XlsxGraft.csproj")
    problems = [problem for nupkg in nupkgs for problem in check(Path(nupkg), found.group(1))]
    for problem in problems:
        print(f"check_nuspec: {problem}", file=sys.stderr)
    print(f"check_nuspec: {len(nupkgs)} nupkgs, {len(problems)} problems")
    return 1 if problems or not nupkgs else 0


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:]))
