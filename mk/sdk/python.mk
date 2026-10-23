# sdk/python — the Python SDK's gates and lockfile (mk/sdk.mk names the values
# a language's file sets and what `sdk_lang` makes of them)

.PHONY: _sdk-image-python _sdk-lint-python _sdk-lint-python-line _sdk-test-python \
        _sdk-test-python-line _sdk-verify-python _sdk-verify-python-line sdk\:lint\:python \
        sdk\:lock\:python sdk\:test\:python sdk\:verify\:python

# The floor first and the latest stable line last: LATEST=1 takes the last and
# sdk:lock:python the first. The base image's Debian layers still move when
# Docker Hub rebuilds the tag a patch release names.
PYTHON_LINES := 3.11 3.12 3.13 3.14 3.15
PYTHON_PATCH_3.11 := 3.11.17
PYTHON_PATCH_3.12 := 3.12.15
PYTHON_PATCH_3.13 := 3.13.17
PYTHON_PATCH_3.14 := 3.14.9
PYTHON_PATCH_3.15 := 3.15.1
# Command line only (Makefile § knobs).
PYTHON_VER :=
PYTHON_IMAGE_INPUTS := $(addprefix sdk/python/,Dockerfile pyproject.toml uv.lock)
$(eval $(call sdk_lang,python,PYTHON))

# Writes sdk/python/uv.lock and does NOT go through `quiet`, like engine:lock.
# It runs on the Dockerfile's `uv` stage, the interpreter and uv without the
# tools: the gate image installs from the lockfile, so it cannot be the one
# that makes it. uv.lock is universal: it resolves for every platform and
# every line requires-python allows, whichever interpreter runs it. It runs on
# the floor only as sdk:lock:ruby does; whether every line installs the lock is
# what that line's gate checks.
sdk\:lock\:python: override PYTHON_VER = $(firstword $(PYTHON_LINES))
sdk\:lock\:python: ## Re-resolve sdk/python/uv.lock after a pyproject.toml edit (then: git add sdk/python/uv.lock)
	@echo "== sdk:lock:python (Python $(PYTHON_PATCH)) =="
	docker build -q --target uv --build-arg PYTHON_VERSION=$(PYTHON_PATCH) \
		-t $(PYTHON_IMAGE)-uv sdk/python >/dev/null
	$(GATE_LOCK) docker run --rm --init --label "xlsx-graft.tree=$(CURDIR)" \
		-v "$(CURDIR):/repo" -w /repo/sdk/python $(PYTHON_IMAGE)-uv uv lock

_sdk-image-python:
	@echo "== sdk python image (Python $(PYTHON_PATCH)) =="
	@docker build -q --build-arg PYTHON_VERSION=$(PYTHON_PATCH) -t $(PYTHON_IMAGE) sdk/python >/dev/null

# Lint, the tests at 100% line coverage, then the wheel as a user gets it:
# built from a copy of the checkout (minus any library left in
# src/xlsx_graft/native/, which the wheel would otherwise ship) with the locked
# hatchling and no isolation, so nothing is downloaded; installed alone into a
# virtual environment of its own; and imported from there with `python -I`,
# from outside the checkout, so neither src/ nor the gate's tools are on the
# path.
sdk\:verify\:python: ## Verify sdk/python on every supported line — ruff + mypy + pytest at 100% line coverage + wheel build/install/load (LATEST=1, PYTHON_VER=<line>)
	@$(call gate,_sdk-verify-python,sdk:verify:python)

_sdk-verify-python: _engine-capi-lib
	@$(call python_each,_sdk-verify-python-line)

_sdk-verify-python-line: _sdk-image-python
	@echo "== sdk python lint + test (Python $(PYTHON_PATCH)) =="
	$(PYTHON_IN_DOCKER) '$(PYTHON_LINT); pytest'
	@echo "== sdk python wheel (build, install, load) (Python $(PYTHON_PATCH)) =="
	$(PYTHON_IN_DOCKER) 'cp -r /repo/sdk/python /tmp/build; \
		rm -rf /tmp/build/src/xlsx_graft/native; \
		cd /tmp/build; \
		python -m build --wheel --no-isolation --outdir /tmp/dist .; \
		/usr/local/bin/python3 -m venv /tmp/user; \
		/tmp/user/bin/python -m pip install --no-index --no-deps --no-cache-dir /tmp/dist/*.whl; \
		cd /tmp; \
		/tmp/user/bin/python -I /repo/sdk/python/script/check_installed_wheel.py /tmp/user /repo/engine/Cargo.toml'

sdk\:test\:python: ## sdk/python pytest at 100% line coverage, on every supported line (LATEST=1, PYTHON_VER=<line>)
	@$(call gate,_sdk-test-python,sdk:test:python)

_sdk-test-python: _engine-capi-lib
	@$(call python_each,_sdk-test-python-line)

_sdk-test-python-line: _sdk-image-python
	@echo "== sdk python test (Python $(PYTHON_PATCH)) =="
	$(PYTHON_IN_DOCKER) 'pytest'

# One command per step under `sh -e`, so the first red one stops the rest.
PYTHON_LINT := ruff format --check .; ruff check .; mypy

# No engine library needed, so no capi-lib: lint runs without a Rust build.
sdk\:lint\:python: ## sdk/python ruff (format + check) and mypy --strict, on every supported line (LATEST=1, PYTHON_VER=<line>)
	@$(call gate,_sdk-lint-python,sdk:lint:python)

_sdk-lint-python:
	@$(call python_each,_sdk-lint-python-line)

_sdk-lint-python-line: _sdk-image-python
	@echo "== sdk python lint (Python $(PYTHON_PATCH)) =="
	$(PYTHON_IN_DOCKER) '$(PYTHON_LINT)'
