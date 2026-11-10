"""Checks what the assembled packages hold, and that no other package is there.

    python3 -I scripts/release/check_packages.py <packages dir> <ruby version> \
        <python version> <cli version> <slug>...

A package is checked by the names of its files, never by running it:

* Every package holds the license files, LICENSE-APACHE, LICENSE-BSD,
  LICENSE-MIT and NOTICE.
* A package for a platform holds the engine of that platform once (the C ABI
  library; the CLI in a CLI archive) and THIRD-PARTY-LICENSES, the licenses of
  the dependencies compiled into it.
* The gem and the wheel for every platform hold no engine and no
  THIRD-PARTY-LICENSES, as nothing of the dependencies is in them.

Reads gems (a tar holding data.tar.gz), wheels and the CLI's zip and tar.gz
with the standard library alone. Every failure is printed, then the exit
status is 1.
"""

from __future__ import annotations

import gzip
import io
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[2]
PLATFORM = ROOT / "scripts" / "release" / "platform.sh"
NOTICES = ("LICENSE-APACHE", "LICENSE-BSD", "LICENSE-MIT", "NOTICE")
THIRD_PARTY = "THIRD-PARTY-LICENSES"
CAPI_NAMES = ("libxlsx_graft_capi.so", "libxlsx_graft_capi.dylib", "xlsx_graft_capi.dll")
CLI_NAMES = ("xlsx-graft", "xlsx-graft.exe")


def column(slug: str, name: str) -> str:
    result = subprocess.run(  # noqa: S603
        [str(PLATFORM), slug, name], check=True, capture_output=True, text=True
    )
    return result.stdout.strip()


def gem_members(path: Path) -> list[str]:
    with tarfile.open(path) as outer:
        data = outer.extractfile("data.tar.gz")
        if data is None:
            return []
        with tarfile.open(fileobj=io.BytesIO(gzip.decompress(data.read()))) as inner:
            return inner.getnames()


def members(path: Path) -> list[str]:
    if path.suffix == ".gem":
        return gem_members(path)
    if path.suffix in (".whl", ".zip"):
        with zipfile.ZipFile(path) as archive:
            return archive.namelist()
    with tarfile.open(path) as tar:
        return tar.getnames()


def check_one(path: Path, engine_names: tuple[str, ...], has_engine: bool) -> list[str]:
    """What is wrong with one package; `has_engine` says what it must hold."""
    found = [PurePosixPath(name).name for name in members(path)]
    problems = [f"{path.name}: no {name}" for name in NOTICES if name not in found]
    has_third_party = THIRD_PARTY in found
    if has_engine != has_third_party:
        what = "no" if has_engine else "a"
        problems.append(f"{path.name}: {what} {THIRD_PARTY}")
    held = [name for name in found if name in CAPI_NAMES + CLI_NAMES]
    wanted = list(engine_names) if has_engine else []
    if sorted(held) != sorted(wanted):
        problems.append(f"{path.name}: holds the engine files {held}, not {wanted}")
    return problems


def main(packages: str, ruby: str, python: str, cli: str, *slugs: str) -> int:
    root = Path(packages)
    expected: dict[Path, tuple[tuple[str, ...], bool]] = {
        root / "ruby" / f"xlsx_graft-{ruby}.gem": ((), False),
        root / "python" / f"xlsx_graft-{python}-py3-none-any.whl": ((), False),
    }
    for slug in slugs:
        capi = column(slug, "capi")
        gem = column(slug, "gem")
        wheel = column(slug, "wheel")
        suffix = "zip" if slug.startswith("win32-") else "tar.gz"
        expected[root / "ruby" / f"xlsx_graft-{ruby}-{gem}.gem"] = ((capi,), True)
        expected[root / "python" / f"xlsx_graft-{python}-py3-none-{wheel}.whl"] = ((capi,), True)
        cli_name = column(slug, "cli")
        expected[root / "cli" / f"xlsx-graft-{cli}-{slug}.{suffix}"] = ((cli_name,), True)

    problems: list[str] = []
    for path, (engine_names, has_engine) in expected.items():
        if path.is_file():
            problems += check_one(path, engine_names, has_engine)
        else:
            problems.append(f"{path.relative_to(root)} is missing")
    present = {p for sub in ("ruby", "python", "cli") for p in (root / sub).glob("*") if p.is_file()}
    for extra in sorted(present - set(expected)):
        if extra.name != "SHA256SUMS":
            problems.append(f"{extra.relative_to(root)} is a package nobody asked for")

    for problem in problems:
        print(f"check_packages: {problem}", file=sys.stderr)
    print(f"check_packages: {len(expected)} packages, {len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    if len(sys.argv) < 6:
        sys.exit(f"usage: {sys.argv[0]} <packages dir> <ruby> <python> <cli> <slug>...")
    sys.exit(main(*sys.argv[1:]))
