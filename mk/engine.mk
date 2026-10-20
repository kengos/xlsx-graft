# engine/ — the Rust workspace: gates and lockfile
#
# Every target defined here is named `engine:<job>` (public) or `_engine-<job>`
# (private, carrying the recipe).

.PHONY: _engine-budget _engine-clippy _engine-coverage _engine-doctest _engine-fmt _engine-lint \
        _engine-test _engine-verify engine\:budget engine\:clippy engine\:coverage \
        engine\:coverage-why engine\:fmt engine\:format engine\:lint engine\:lock \
        engine\:test engine\:update engine\:verify

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
# coverage-why saying "every line is covered").
engine\:verify: ## Verify engine/ — budget + fmt + clippy + tests at 100% line coverage + doctests
	@$(call gate,_engine-verify,engine:verify)

_engine-verify: _engine-lint _engine-coverage _engine-doctest

_engine-doctest:
	@echo "== doctest =="
	$(CARGO_IN_DOCKER) 'cargo test --workspace --doc --locked $(CARGO_JOBS)'

engine\:lint: ## engine/ line budget + //! headers + fmt --check + clippy -D warnings
	@$(call gate,_engine-lint,engine:lint)

_engine-lint: _engine-budget _engine-fmt _engine-clippy

engine\:budget: ## .rs line budget + //! header check (scripts/check-line-budget.sh)
	@$(call gate,_engine-budget,engine:budget)

_engine-budget:
	@echo "== line budget =="
	@scripts/check-line-budget.sh

engine\:fmt: ## cargo fmt --check
	@$(call gate,_engine-fmt,engine:fmt)

_engine-fmt:
	@echo "== fmt =="
	$(CARGO_IN_DOCKER) 'rustup component add rustfmt >/dev/null; \
		cargo fmt --all -- --check'

engine\:format: ## cargo fmt (apply formatting)
	$(CARGO_IN_DOCKER) 'rustup component add rustfmt >/dev/null; \
		cargo fmt --all'

engine\:clippy: ## cargo clippy -D warnings (JOBS=N caps parallelism)
	@$(call gate,_engine-clippy,engine:clippy)

_engine-clippy:
	@echo "== clippy =="
	$(CARGO_IN_DOCKER) 'rustup component add clippy >/dev/null; \
		cargo clippy --workspace --all-targets --all-features --locked $(CARGO_JOBS) -- -D warnings'

# P=<crate> and F=<name filter> narrow the run for the edit-run-edit loop
# (`make engine:test P=xlsx-graft-core`, `… F=version`, or both). A narrowed
# run proves nothing about what it skipped: finish with engine:verify.
engine\:test: ## cargo test --workspace --locked (P=<crate> F=<filter> narrow it)
	@$(call gate,_engine-test,engine:test)

_engine-test:
	@echo "== test$(if $(P), P=$(P))$(if $(F), F=$(F)) =="
	$(CARGO_IN_DOCKER) 'cargo test $(if $(P),$(foreach p,$(P),-p $(p)),--workspace) --locked $(CARGO_JOBS) $(F)'

## ---- coverage ----------------------------------------------------------

# Measured over the WHOLE workspace, not per crate. The `rm -f lcov.info` is
# load-bearing: when a test does not compile or fails, llvm-cov writes no report, and
# coverage-why.sh would otherwise reprint the PREVIOUS run's uncovered lines.
engine\:coverage: ## cargo llvm-cov, blocking at 100% lines (names the offending lines on failure)
	@$(call gate,_engine-coverage,engine:coverage)

_engine-coverage:
	@echo "== coverage (100% lines) =="
	@$(CARGO_IN_DOCKER) 'rm -f lcov.info; \
		rustup component add llvm-tools-preview >/dev/null; \
		cargo llvm-cov --version 2>/dev/null | grep -qx "cargo-llvm-cov $(LLVM_COV_VERSION)" \
			|| cargo install cargo-llvm-cov --locked --version $(LLVM_COV_VERSION); \
		cargo llvm-cov --workspace --locked --fail-under-lines 100 $(CARGO_JOBS) \
			--lcov --output-path lcov.info' \
	|| { code=$$?; \
	     echo; echo "-- which lines? (scripts/coverage-why.sh) --"; \
	     $(MAKE) --no-print-directory engine:coverage-why || true; \
	     exit $$code; }

engine\:coverage-why: ## Name the lines that failed the coverage gate (reads engine/lcov.info; no re-run)
	@scripts/coverage-why.sh
