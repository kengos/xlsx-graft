"""Checks what the assembled packages hold, and that no other package is there.

    python3 -I scripts/release/check_packages.py <packages dir> <slug>...
    python3 -I scripts/release/check_packages.py --crates <dir of .crate files>

A package is checked by the names of its files, never by running it:

* Every package holds the license files, LICENSE-APACHE, LICENSE-BSD,
  LICENSE-MIT and NOTICE.
* A package with the engine in it holds the engine of each platform it is for
  at the exact path the SDK looks it up at, and THIRD-PARTY-LICENSES, the
  licenses of the dependencies compiled into the engine.
* A package with no engine (the gem and the wheel for every platform, the npm
  package that names the platform packages, the Java jar of the classes) holds
  no engine file and no THIRD-PARTY-LICENSES, as nothing of the dependencies is
  in it.

With --crates, the packages are the .crate files of the crates published to
crates.io (every workspace crate that does not say `publish = false`), which
hold source alone: the license files, and no engine and no THIRD-PARTY-LICENSES.

The versions are read from the SDKs' own files, so the names of the packages
a release makes are the ones expected. Reads gems (a tar holding
data.tar.gz), wheels, nupkgs, jars and zips, and tarballs, with the standard
library alone. Every failure is printed, then the exit status is 1.
"""

from __future__ import annotations

import gzip
import io
import json
import re
import subprocess
import sys
import tarfile
import tomllib
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[2]
PLATFORM = ROOT / "scripts" / "release" / "platform.sh"
NOTICES = ("LICENSE-APACHE", "LICENSE-BSD", "LICENSE-MIT", "NOTICE")
THIRD_PARTY = "THIRD-PARTY-LICENSES"
# The files an engine is: the C ABI library of each platform, the CLI of each
# platform (Windows') and the Node.js addon.
ENGINE_NAMES = (
    "libxlsx_graft_capi.so",
    "libxlsx_graft_capi.dylib",
    "xlsx_graft_capi.dll",
    "xlsx-graft",
    "xlsx-graft.exe",
    "xlsx_graft.node",
)

JS_VERSION = r'"version": "([^"]+)"'

# A package: where it is under the packages directory, and the exact paths in
# it of the engine files it holds (none for a package with no engine).
Expected = dict[Path, list[str]]


def column(slug: str, name: str) -> str:
    result = subprocess.run(  # noqa: S603
        [str(PLATFORM), slug, name], check=True, capture_output=True, text=True
    )
    return result.stdout.strip()


def read(path: str, pattern: str) -> str:
    """The first group of `pattern` in a file of the repository."""
    found = re.search(pattern, (ROOT / path).read_text(encoding="utf-8"), re.MULTILINE)
    if found is None:
        sys.exit(f"check_packages: no {pattern} in {path}")
    return found.group(1)


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
    if path.suffix in (".whl", ".zip", ".nupkg", ".jar"):
        with zipfile.ZipFile(path) as archive:
            return archive.namelist()
    with tarfile.open(path) as tar:
        return tar.getnames()


def check_one(path: Path, engine_paths: list[str], top: str = "") -> list[str]:
    """What is wrong with one package; `engine_paths` says what it must hold.

    `top` is the directory the package's files are in (a .crate holds its files
    under `<name>-<version>/`); the license files are looked for there.
    """
    names = members(path)
    found = [PurePosixPath(name).name for name in names]
    # With no `top`, the license files may be at any depth (a wheel puts them
    # in `.dist-info/licenses/`).
    held_notices = set(names) if top else set(found)
    problems = [f"{path.name}: no {top}{name}" for name in NOTICES if f"{top}{name}" not in held_notices]
    has_engine = bool(engine_paths)
    if has_engine != (THIRD_PARTY in found):
        problems.append(f"{path.name}: {'no' if has_engine else 'a'} {THIRD_PARTY}")
    held = sorted(name for name in names if PurePosixPath(name).name in ENGINE_NAMES)
    if held != sorted(engine_paths):
        problems.append(f"{path.name}: holds the engine files {held}, not {sorted(engine_paths)}")
    return problems


def check_optional_dependencies(path: Path, version: str, slugs: tuple[str, ...]) -> list[str]:
    """The npm package of the classes names every platform package at its version."""
    with tarfile.open(path) as tar:
        manifest_file = tar.extractfile("package/package.json")
        manifest = json.load(manifest_file) if manifest_file else {}
    wanted = {column(slug, "npm"): version for slug in slugs}
    held = manifest.get("optionalDependencies")
    if held != wanted:
        return [f"{path.name}: optionalDependencies are {held}, not {wanted}"]
    return []


def expected_packages(root: Path, slugs: tuple[str, ...]) -> Expected:
    ruby = read("sdk/ruby/lib/xlsx_graft/version.rb", r'VERSION = "([^"]+)"')
    python = read("sdk/python/src/xlsx_graft/_version.py", r'^__version__ = "([^"]+)"')
    dotnet = read("sdk/dotnet/XlsxGraft/XlsxGraft.csproj", r"<Version>([^<]+)</Version>")
    jvm = read("sdk/jvm/gradle.properties", r"^version=(\S+)")
    js = read("sdk/js/package.json", JS_VERSION)
    cli = read("engine/Cargo.toml", r'^\[workspace\.package\][^\[]*?^version = "([^"]+)"')

    jvm_dir = root / "jvm" / "io" / "github" / "kengos" / "xlsx-graft" / jvm
    expected: Expected = {
        root / "ruby" / f"xlsx_graft-{ruby}.gem": [],
        root / "python" / f"xlsx_graft-{python}-py3-none-any.whl": [],
        jvm_dir / f"xlsx-graft-{jvm}.jar": [],
        root / "js" / f"xlsx-graft-{js}.tgz": [],
        root / "dotnet" / f"XlsxGraft.{dotnet}.nupkg": [],
    }
    for slug in slugs:
        capi, cli_name = column(slug, "capi"), column(slug, "cli")
        npm = column(slug, "npm").lstrip("@").replace("/", "-")
        suffix = "zip" if slug.startswith("win32-") else "tar.gz"
        expected[root / "ruby" / f"xlsx_graft-{ruby}-{column(slug, 'gem')}.gem"] = [
            f"lib/xlsx_graft/native/{capi}"
        ]
        wheel = f"xlsx_graft-{python}-py3-none-{column(slug, 'wheel')}.whl"
        expected[root / "python" / wheel] = [f"xlsx_graft/native/{capi}"]
        expected[root / "cli" / f"xlsx-graft-{cli}-{slug}.{suffix}"] = [cli_name]
        jna = column(slug, "jna")
        expected[jvm_dir / f"xlsx-graft-{jvm}-{jna}.jar"] = [f"native/{jna}/{capi}"]
        expected[root / "js" / f"{npm}-{js}.tgz"] = ["package/xlsx_graft.node"]
        nupkg = root / "dotnet" / f"XlsxGraft.{dotnet}.nupkg"
        expected[nupkg] = [*expected[nupkg], f"runtimes/{column(slug, 'rid')}/native/{capi}"]
    return expected


def expected_crates(root: Path) -> Expected:
    """The .crate of every published workspace crate; source only, so no engine."""
    workspace = tomllib.loads((ROOT / "engine" / "Cargo.toml").read_text(encoding="utf-8"))
    version = workspace["workspace"]["package"]["version"]
    expected: Expected = {}
    for member in workspace["workspace"]["members"]:
        manifest = ROOT / "engine" / member / "Cargo.toml"
        package = tomllib.loads(manifest.read_text(encoding="utf-8"))["package"]
        if package.get("publish") is False:
            continue
        expected[root / f"{package['name']}-{version}.crate"] = []
    return expected


def report(root: Path, expected: Expected, kinds: tuple[str, ...], problems: list[str]) -> int:
    """Adds the packages nobody asked for to `problems`, prints them all, returns the exit status."""
    present = {p for kind in kinds for p in root.rglob(kind) if p.is_file()}
    present = {p for p in present if not p.name.endswith(("-sources.jar", "-javadoc.jar"))}
    for extra in sorted(present - set(expected)):
        problems.append(f"{extra.relative_to(root)} is a package nobody asked for")

    for problem in problems:
        print(f"check_packages: {problem}", file=sys.stderr)
    print(f"check_packages: {len(expected)} packages, {len(problems)} problems")
    return 1 if problems else 0


def main(packages: str, *slugs: str) -> int:
    root = Path(packages)
    expected = expected_packages(root, slugs)

    problems: list[str] = []
    for path, engine_paths in expected.items():
        if path.is_file():
            problems += check_one(path, engine_paths)
            if path.name == f"xlsx-graft-{read('sdk/js/package.json', JS_VERSION)}.tgz":
                problems += check_optional_dependencies(path, read("sdk/js/package.json", JS_VERSION), slugs)
        else:
            problems.append(f"{path.relative_to(root)} is missing")

    # A package of a kind this script checks, that nobody asked for. A Maven
    # repository holds files that are no package (POMs, checksums, sources and
    # javadoc jars), so only the jars named like the classes' jar are looked at.
    kinds = ("*.gem", "*.whl", "*.nupkg", "*.tgz", "*.zip", "*.tar.gz", "xlsx-graft-*.jar")
    return report(root, expected, kinds, problems)


def main_crates(directory: str) -> int:
    root = Path(directory)
    expected = expected_crates(root)
    problems: list[str] = []
    for path, engine_paths in expected.items():
        if path.is_file():
            problems += check_one(path, engine_paths, top=f"{path.name.removesuffix('.crate')}/")
        else:
            problems.append(f"{path.relative_to(root)} is missing")
    return report(root, expected, ("*.crate",), problems)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--crates":
        sys.exit(main_crates(sys.argv[2]))
    if len(sys.argv) < 3:
        sys.exit(f"usage: {sys.argv[0]} <packages dir> <slug>... | --crates <dir>")
    sys.exit(main(*sys.argv[1:]))
