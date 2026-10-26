# engine/ — the Rust workspace: gates and lockfile
#
# Every target defined here is named `engine:<job>` (public) or `_engine-<job>`
# (private, carrying the recipe).

.PHONY: _engine-budget _engine-capi-abi _engine-capi-lib _engine-clippy _engine-coverage _engine-doctest _engine-fmt \
        _engine-lint _engine-napi _engine-test _engine-toolchain _engine-verify engine\:budget engine\:capi-abi engine\:capi-lib engine\:clippy \
        engine\:coverage engine\:coverage-why engine\:fmt engine\:format engine\:lint \
        engine\:lock engine\:napi engine\:test engine\:update engine\:verify

## ---- lockfile ----------------------------------------------------------
#
# These WRITE engine/Cargo.lock and deliberately do NOT go through `quiet`:
# you want to read what moved. They prove nothing — run engine:verify after.
#
#   engine:lock    re-resolve after a MANIFEST edit. A dependency that already
#                  satisfies its range does not move. `cargo metadata` resolves
#                  without compiling, so this is seconds rather than a build.
#   engine:update  bump to the newest release each range still allows. It
#                  moves things you did not name, so read the lockfile diff.

engine\:lock: ## Re-resolve engine/Cargo.lock after a Cargo.toml edit (then: git add engine/Cargo.lock)
	@echo "== engine:lock =="
	$(CARGO_IN_DOCKER) 'cargo metadata --format-version 1 >/dev/null'

engine\:update: ## Bump engine/ deps within their Cargo.toml ranges (then: git add engine/Cargo.lock)
	@echo "== engine:update =="
	$(CARGO_IN_DOCKER) 'cargo update'

## ---- engine -----------------------------------------------------------

# The scope's WHOLE bar; engine:budget / engine:fmt / engine:clippy /
# engine:test are the fast slices to iterate on. Coverage runs the unit and
# integration tests; llvm-cov measures doctests only on nightly, so they run as
# their own member (kept out of coverage, where a failing doctest would leave
# coverage-why saying "every line is covered"). capi-abi is the one check on
# the shared library itself, which no test links; it runs before coverage
# because it is the cheaper of the two. budget needs no Docker, so it fails
# before the first pull; toolchain comes next, since every member after it is
# only worth as much as the image it ran in. budget runs once, not again
# inside lint: make builds a prerequisite once per run.
engine\:verify: ## Verify engine/ — budget + toolchain pins + fmt + clippy + tests at 100% line coverage + doctests + C ABI symbols
	@$(call gate,_engine-verify,engine:verify)

_engine-verify: _engine-budget _engine-toolchain _engine-lint _engine-capi-abi _engine-coverage \
                _engine-doctest

# The pinned digest holds the versions TOOLCHAIN_TAG names. A digest copied
# from the wrong run, or a tag left behind when the digest moved, fails here
# rather than passing every gate on a toolchain the Makefile does not name.
_engine-toolchain:
	@echo "== toolchain image =="
	$(CARGO_IN_DOCKER) 'have="$$(rustc --version | cut -d" " -f2)-llvmcov$$(cargo llvm-cov --version | cut -d" " -f2)"; \
		[ "$$have" = "$(TOOLCHAIN_TAG)" ] \
			|| { echo "toolchain: the pinned image holds $$have, but TOOLCHAIN_TAG says $(TOOLCHAIN_TAG)" >&2; exit 1; }'

_engine-doctest:
	@echo "== doctest =="
	$(CARGO_IN_DOCKER) 'cargo test --workspace --doc --locked $(CARGO_JOBS)'

engine\:lint: ## engine/ line budget + //! headers + crate lint opt-in + fmt --check + clippy -D warnings
	@$(call gate,_engine-lint,engine:lint)

_engine-lint: _engine-budget _engine-fmt _engine-clippy

engine\:budget: ## .rs line budget + //! headers + every crate opts into the workspace lints + toolchain pins (no Docker)
	@$(call gate,_engine-budget,engine:budget)

_engine-budget:
	@echo "== line budget =="
	@scripts/check-line-budget.sh
	@echo "== crate lints =="
	@scripts/check-crate-lints.sh
	@echo "== toolchain pins =="
	@scripts/check-toolchain-pins.sh

engine\:fmt: ## cargo fmt --check
	@$(call gate,_engine-fmt,engine:fmt)

_engine-fmt:
	@echo "== fmt =="
	$(CARGO_IN_DOCKER) 'cargo fmt --all -- --check'

engine\:format: ## cargo fmt (apply formatting)
	$(CARGO_IN_DOCKER) 'cargo fmt --all'

engine\:clippy: ## cargo clippy -D warnings (JOBS=N caps parallelism)
	@$(call gate,_engine-clippy,engine:clippy)

_engine-clippy:
	@echo "== clippy =="
	$(CARGO_IN_DOCKER) 'cargo clippy --workspace --all-targets --all-features --locked $(CARGO_JOBS) -- -D warnings'

# P=<crate> and F=<name filter> narrow the run for the edit-run-edit loop
# (`make engine:test P=xlsx-graft-core`, `… F=version`, or both). A narrowed
# run SKIPS the C ABI symbol check below and proves nothing about what it
# skipped: finish with engine:verify. Command line only (Makefile § knobs).
P :=
F :=

engine\:test: ## cargo test --workspace --locked + the C ABI symbol check (P=<crate> F=<filter> narrow it)
	@$(call gate,_engine-test,engine:test)

_engine-test:
	@echo "== test$(if $(P), P=$(P))$(if $(F), F=$(F)) =="
	$(CARGO_IN_DOCKER) 'cargo test $(if $(P),$(foreach p,$(P),-p $(p)),--workspace) --locked $(CARGO_JOBS) $(F)'
	@$(if $(or $(P),$(F)),echo "== capi abi SKIPPED (narrowed run) ==",:)
ifeq ($(strip $(P)$(F)),)
	@$(MAKE) --no-print-directory _engine-capi-abi
endif

## ---- capi (the C ABI shared library the FFI SDKs load) -----------------

# `cargo test` links only the rlib, so a Rust test stays green when an export
# loses `#[unsafe(no_mangle)]` or the hand-written header names a function the
# library does not export. This builds the cdylib and compares its dynamic
# symbol table with include/xlsx_graft.h (scripts/check-capi-symbols.sh). nm
# comes with the toolchain image, so both steps run in the same container.
CAPI_SO     := target/debug/libxlsx_graft_capi.so
CAPI_HEADER := capi/include/xlsx_graft.h

engine\:capi-abi: ## Build the C ABI library and check its exports match include/xlsx_graft.h
	@$(call gate,_engine-capi-abi,engine:capi-abi)

_engine-capi-abi:
	@echo "== capi abi =="
	$(CARGO_IN_DOCKER) 'cargo build -p xlsx-graft-capi --locked $(CARGO_JOBS) \
		&& ../scripts/check-capi-symbols.sh $(CAPI_SO) $(CAPI_HEADER)'

# The SDKs load a RELEASE build of the same library, for the architecture of
# the Docker host (arm64 Linux on an Apple Silicon Mac), from the gitignored
# CAPI_LOCAL. The directory is emptied first so a library left by an earlier
# run never stands in for this one, and it must end up holding exactly one
# library: an SDK picks it up without naming its extension. The symbol check
# then runs on the copy the SDKs will load, not on the debug build above.
CAPI_LOCAL := dist/capi/local

engine\:capi-lib: ## Build the release C ABI library for the Docker host's arch into dist/capi/local (the SDKs load it)
	@$(call gate,_engine-capi-lib,engine:capi-lib)

_engine-capi-lib:
	@echo "== capi lib (release, Docker host arch) =="
	$(CARGO_IN_DOCKER) 'out=/repo/$(CAPI_LOCAL); rm -rf "$$out"; mkdir -p "$$out"; \
		cargo build -p xlsx-graft-capi --release --locked $(CARGO_JOBS); \
		for lib in target/release/libxlsx_graft_capi.so target/release/libxlsx_graft_capi.dylib \
		           target/release/xlsx_graft_capi.dll; do \
			if [ -f "$$lib" ]; then cp "$$lib" "$$out/"; fi; \
		done; \
		count=$$(ls "$$out" | wc -l); \
		[ "$$count" -eq 1 ] || { echo "capi-lib: expected one library in $(CAPI_LOCAL)/, found $$count" >&2; ls "$$out" >&2; exit 1; }; \
		lib=$$(ls "$$out"); \
		../scripts/check-capi-symbols.sh "$$out/$$lib" $(CAPI_HEADER); \
		echo "engine library: $(CAPI_LOCAL)/$$lib"'

## ---- napi (the Node.js addon the JS SDK loads) -------------------------

# Node.js cannot load the C ABI library (no FFI on the lines the JS SDK
# supports), so the JS SDK loads an addon that links it statically. This
# builds the addon in release, with its N-API shim (the `shim` feature, which
# no test or coverage run builds), for the Docker host's arch, and renames the
# cdylib to the `.node` Node.js `require()`s. As in engine:capi-lib, the
# directory is emptied first so an addon left by an earlier run never stands
# in for this one. The build runs in the Linux toolchain container, so the
# cdylib is always a `.so`. Not part of engine:verify: whether the addon loads
# and answers is what the JS SDK's gate checks, through Node.js itself.
NAPI_LOCAL := dist/napi/local

engine\:napi: ## Build the release Node.js addon for the Docker host's arch into dist/napi/local (the JS SDK loads it)
	@$(call gate,_engine-napi,engine:napi)

_engine-napi:
	@echo "== napi addon (release, Docker host arch) =="
	$(CARGO_IN_DOCKER) 'out=/repo/$(NAPI_LOCAL); rm -rf "$$out"; mkdir -p "$$out"; \
		cargo build -p xlsx-graft-napi --release --features shim --locked $(CARGO_JOBS); \
		cp target/release/libxlsx_graft_napi.so "$$out/xlsx_graft.node"; \
		echo "node addon: $(NAPI_LOCAL)/xlsx_graft.node"'

## ---- coverage ----------------------------------------------------------

# Measured over the WHOLE workspace, not per crate. The `rm -f lcov.info` is
# load-bearing: when a test does not compile or fails, llvm-cov writes no report, and
# coverage-why.sh would otherwise reprint the PREVIOUS run's uncovered lines.
engine\:coverage: ## cargo llvm-cov, blocking at 100% lines (names the offending lines on failure)
	@$(call gate,_engine-coverage,engine:coverage)

_engine-coverage:
	@echo "== coverage (100% lines) =="
	@$(CARGO_IN_DOCKER) 'rm -f lcov.info; \
		cargo llvm-cov --workspace --locked --fail-under-lines 100 $(CARGO_JOBS) \
			--lcov --output-path lcov.info' \
	|| { code=$$?; \
	     echo; echo "-- which lines? (scripts/coverage-why.sh) --"; \
	     $(MAKE) --no-print-directory engine:coverage-why || true; \
	     exit $$code; }

engine\:coverage-why: ## Name the lines that failed the coverage gate (reads engine/lcov.info; no re-run)
	@scripts/coverage-why.sh
