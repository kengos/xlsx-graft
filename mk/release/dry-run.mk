# release dry run — what a release would publish is checked on every pull
# request (the registry's package and metadata rules, the license files in
# each package, the versions), and nothing is published
#
# Public targets `version:check`, `release:dry-run` and
# `release:dry-run:<registry>`, with the recipes in `_<name>`, like
# mk/engine.mk. `release:dry-run` is a member of `verify` (and of CI's matrix,
# `make ci:matrix`): a form that a registry would refuse then fails the pull
# request that broke it, not the release. A dry run uses no token and writes to
# no registry; it reads the network only to fetch dependencies.
#
#   version:check             the versions of the crates' path dependencies and
#                             of the Ruby, Python, .NET, Java and JS manifests
#                             are the workspace's (scripts/check-versions.sh;
#                             no Docker)
#   release:dry-run:crates    `cargo publish --dry-run` of the crates on
#                             crates.io (the CLI, the MCP server and the engine
#                             crates they build on; capi and napi go in the SDK
#                             packages), then what each .crate holds
#                             (scripts/release/check_packages.py --crates)
#   release:dry-run           every registry's dry run, and version:check
#
# A .crate holds source alone, so it holds the license files (LICENSE-APACHE,
# LICENSE-BSD, LICENSE-MIT, NOTICE) and no THIRD-PARTY-LICENSES: the licenses of
# the dependencies are in the packages that hold the engine binary.

.PHONY: _release-dry-run _release-dry-run-crates _release-dry-run-crates-check _version-check \
        release\:dry-run release\:dry-run\:crates version\:check

RELEASE_DRY_RUN := dist/release/dry-run
CRATES_NOT_PUBLISHED := --exclude xlsx-graft-capi --exclude xlsx-graft-napi --exclude xlsx-graft-fixtures-gen

version\:check: ## Fail when a package manifest's version is not the workspace's (no Docker)
	@$(call gate,_version-check,version:check)

_version-check:
	@echo "== version check =="
	@scripts/check-versions.sh

release\:dry-run: ## Every registry's dry run and version:check, nothing published (a member of verify)
	@$(call gate,_release-dry-run,release:dry-run)

_release-dry-run: _version-check _release-dry-run-crates

release\:dry-run\:crates: ## cargo publish --dry-run of the crates published to crates.io, and the files each .crate holds
	@$(call gate,_release-dry-run-crates,release:dry-run:crates)

# --allow-dirty: a contributor's tree is rarely clean; the release workflow
# publishes a tag's checkout without it. --workspace skips a crate that says
# `publish = false`. `cargo publish --dry-run` builds each .crate to verify it
# and does not keep it, so `cargo package --no-verify` writes the .crate files
# that check_packages.py reads (--no-verify: the dry run has compiled them).
# Unlike publish, `cargo package
# --workspace` does not skip a crate that says `publish = false`, so the three
# are excluded by name; the check then expects exactly the crates that are
# published, so a crate left out here or added without its `publish = false`
# fails it.
_release-dry-run-crates:
	@echo "== release dry run: crates.io =="
	$(CARGO_IN_DOCKER) 'rm -rf target/package; \
		cargo publish --workspace --dry-run --locked --allow-dirty $(CARGO_JOBS); \
		cargo package --workspace $(CRATES_NOT_PUBLISHED) --no-verify --locked --allow-dirty $(CARGO_JOBS); \
		out=/repo/$(RELEASE_DRY_RUN)/crates; rm -rf "$$out"; mkdir -p "$$out"; \
		cp target/package/*.crate "$$out/"'
	@$(MAKE) --no-print-directory _release-dry-run-crates-check PYTHON_VER=$(lastword $(PYTHON_LINES))

_release-dry-run-crates-check: _sdk-image-python
	@echo "== release dry run: what each .crate holds =="
	$(PYTHON_IN_DOCKER) 'python -I /repo/scripts/release/check_packages.py --crates /repo/$(RELEASE_DRY_RUN)/crates'
