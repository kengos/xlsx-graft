"""What every test shares: the engine library and the engine's version."""

from __future__ import annotations

import os
import tomllib
from pathlib import Path

import pytest

from xlsx_graft.library import ENV_VAR

REPO_ROOT = Path(__file__).resolve().parents[3]


@pytest.fixture(scope="session")
def engine_library() -> str:
    """The library `make engine:capi-lib` built; the gate points
    XLSX_GRAFT_LIBRARY at it.
    """
    return os.environ[ENV_VAR]


@pytest.fixture(scope="session")
def engine_version() -> str:
    cargo_toml = tomllib.loads((REPO_ROOT / "engine" / "Cargo.toml").read_text(encoding="utf-8"))
    version: str = cargo_toml["workspace"]["package"]["version"]
    return version
