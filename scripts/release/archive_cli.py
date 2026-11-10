"""Archives the CLI of each platform, for a GitHub Release's assets.

    python3 -I scripts/release/archive_cli.py <native dir> <out dir> <version> <slug>...

Reads  <native dir>/<slug>/<the CLI binary>
Writes <out dir>/xlsx-graft-<version>-<slug>.zip   (Windows)
       <out dir>/xlsx-graft-<version>-<slug>.tar.gz (the others)

An archive holds the CLI, the license files and THIRD-PARTY-LICENSES, which
the binary carries the dependencies' code of. Every entry has a fixed owner,
mode and time, so the same input makes the same archive, and the CLI is
executable whatever mode the workflow's artifact kept (an artifact drops it).
"""

from __future__ import annotations

import gzip
import io
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TABLE = ROOT / "scripts" / "release" / "platform.sh"
LICENSE_FILES = ("LICENSE-APACHE", "LICENSE-BSD", "LICENSE-MIT", "NOTICE", "THIRD-PARTY-LICENSES")
# 1980-01-01, the earliest time a zip can hold.
FIXED_ZIP_TIME = (1980, 1, 1, 0, 0, 0)


def column(slug: str, name: str) -> str:
    result = subprocess.run(  # noqa: S603
        [str(TABLE), slug, name], check=True, capture_output=True, text=True
    )
    return result.stdout.strip()


def entries(native: Path, slug: str) -> list[tuple[str, bytes, int]]:
    """The files of one archive: name, content and mode."""
    cli = column(slug, "cli")
    files = [(cli, (native / slug / cli).read_bytes(), 0o755)]
    files += [(name, (ROOT / name).read_bytes(), 0o644) for name in LICENSE_FILES]
    return files


def write_tar(path: Path, files: list[tuple[str, bytes, int]]) -> None:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.PAX_FORMAT) as tar:
        for name, data, mode in files:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = mode
            info.mtime = 0
            tar.addfile(info, io.BytesIO(data))
    with path.open("wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as gz:
        gz.write(buffer.getvalue())


def write_zip(path: Path, files: list[tuple[str, bytes, int]]) -> None:
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data, mode in files:
            info = zipfile.ZipInfo(name, FIXED_ZIP_TIME)
            info.external_attr = mode << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data)


def main(native: str, out: str, version: str, *slugs: str) -> None:
    out_dir = Path(out)
    out_dir.mkdir(parents=True, exist_ok=True)
    for slug in slugs:
        files = entries(Path(native), slug)
        stem = f"xlsx-graft-{version}-{slug}"
        if slug.startswith("win32-"):
            write_zip(out_dir / f"{stem}.zip", files)
        else:
            write_tar(out_dir / f"{stem}.tar.gz", files)
        print(f"  cli: {slug}")


if __name__ == "__main__":
    if len(sys.argv) < 5:
        sys.exit(f"usage: {sys.argv[0]} <native dir> <out dir> <version> <slug>...")
    main(*sys.argv[1:])
