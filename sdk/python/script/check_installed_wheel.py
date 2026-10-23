"""Run by `make sdk:verify:python` after it installs the built wheel into a
virtual environment of its own: imports the package from there, with no
checkout on the path (run it with `python -I` from outside the checkout), and
checks it reaches the engine.

    python -I script/check_installed_wheel.py <venv-dir> <engine/Cargo.toml>
"""

from __future__ import annotations

import sys
import tomllib
from importlib import metadata
from pathlib import Path

import xlsx_graft


def main(venv: str, cargo_toml: str) -> None:
    workspace = tomllib.loads(Path(cargo_toml).read_text(encoding="utf-8"))["workspace"]
    engine = workspace["package"]["version"]

    where = Path(xlsx_graft.__file__).resolve()
    if not where.is_relative_to(Path(venv).resolve()):
        sys.exit(f"imported xlsx_graft from {where}, not from {venv}")

    distribution = metadata.distribution("xlsx-graft")
    requires = distribution.requires or []
    if requires:
        sys.exit(f"xlsx-graft depends at run time on {requires}, not on nothing")

    wheel = (distribution.read_text("WHEEL") or "").splitlines()
    tags = [line for line in wheel if line.startswith("Tag:")]
    if tags != ["Tag: py3-none-any"]:
        sys.exit(f"the wheel is tagged {tags}, not py3-none-any")

    files = [str(file) for file in distribution.files or []]
    if "xlsx_graft/py.typed" not in files:
        sys.exit("the wheel ships no xlsx_graft/py.typed")

    result = xlsx_graft.version()
    if not result.success:
        sys.exit(f"xlsx_graft.version() failed: {result.diagnostics}")
    if result.value != engine:
        sys.exit(f"xlsx_graft.version() is {result.value}, the engine is {engine}")

    print(
        f"installed xlsx-graft {xlsx_graft.__version__} from {where.parent}"
        f" loads engine {result.value}"
    )


if __name__ == "__main__":
    main(*sys.argv[1:])
