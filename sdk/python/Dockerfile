# The Python SDK's gate environment: `make sdk:*:python` builds and runs this
# image, so nobody needs Python installed. The build context is sdk/python.
#
# The image holds the interpreter, uv and the locked tools, and nothing else.
# The engine library is NOT copied in: the gate mounts the repository at /repo
# and points XLSX_GRAFT_LIBRARY at the library `make engine:capi-lib` built
# there, so changing the engine never rebuilds this image. That library is
# built on bookworm, so this is bookworm too: glibc loads a library built
# against the same or an older glibc, and a musl base would load nothing.
#
# PYTHON_VERSION has no default on purpose. mk/sdk/python.mk maps PYTHON_VER
# (3.11, …) to the patch release, and it is the one place that number is
# written.
ARG PYTHON_VERSION

# `sdk:lock:python` runs this stage alone: it has to make uv.lock, so it cannot
# install from it.
FROM python:${PYTHON_VERSION}-slim-bookworm AS uv

# uv is pinned here and nowhere else.
COPY --from=ghcr.io/astral-sh/uv:0.13.5 /uv /usr/local/bin/uv

# uv uses this image's interpreter and never downloads another one; the tools
# go to /venv, outside /repo (the mount would hide them).
ENV UV_PYTHON=/usr/local/bin/python3 \
    UV_PYTHON_DOWNLOADS=never \
    UV_PROJECT_ENVIRONMENT=/venv \
    UV_NO_CACHE=1

FROM uv

# Only the files that decide the tools. Editing src/ or tests/ then never
# re-runs `uv sync`, and the project itself is not installed: the tests import
# it from the mounted src/, and the gate builds and installs the wheel apart.
# --locked, not --frozen: a pyproject.toml edit without `make sdk:lock:python`
# fails here rather than gating on the stale lock.
WORKDIR /locked
COPY pyproject.toml uv.lock ./
RUN uv sync --locked --no-install-project

# Nothing a gate runs writes into the mounted checkout, which it would leave
# owned by root: no __pycache__, and the caches of mypy, ruff and coverage go
# to /tmp (pytest's is off in pyproject.toml).
ENV PATH=/venv/bin:$PATH \
    PYTHONDONTWRITEBYTECODE=1 \
    MYPY_CACHE_DIR=/tmp/mypy-cache \
    RUFF_NO_CACHE=true \
    COVERAGE_FILE=/tmp/.coverage \
    PIP_DISABLE_PIP_VERSION_CHECK=1

WORKDIR /repo/sdk/python
