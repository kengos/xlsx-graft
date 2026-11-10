# engine/ — the licenses of the dependencies: cargo-deny's policy and the
# committed list of them
#
# Public targets `engine:deny`, `engine:licenses`, `engine:licenses-write` and
# `engine:advisories`, with the recipes in `_engine-<job>`, like mk/engine.mk.
# engine:deny and engine:licenses are members of engine:verify; engine:advisories
# is not (see its comment below).

.PHONY: _engine-advisories _engine-deny _engine-licenses \
        engine\:advisories engine\:deny engine\:licenses engine\:licenses-write

## ---- cargo-deny: the licenses, duplicates and sources of the dependencies --

# engine/deny.toml is the policy. `licenses bans sources` read only Cargo.lock
# and engine/deny.toml (cargo-deny downloads crate metadata, never a database),
# so their result changes only with a commit that changes Cargo.lock,
# engine/deny.toml or TOOLCHAIN_DIGEST (the cargo-deny version).
engine\:deny: ## cargo deny check licenses bans sources (engine/deny.toml; does not fetch the advisory database)
	@$(call gate,_engine-deny,engine:deny)

_engine-deny:
	@echo "== cargo deny =="
	$(CARGO_IN_DOCKER) 'cargo deny --locked check licenses bans sources'

# The security advisories of the RustSec database, which cargo-deny downloads
# with git into the cargo cache volume. Not a member of engine:verify: the
# result depends on the day, not on the commit, so a new advisory would make the
# gate fail in every pull request and on main with no change of theirs, and a
# failed download would too. .github/workflows/advisories.yml runs it (not a
# required check).
engine\:advisories: ## cargo deny check advisories (downloads the RustSec database; not in engine:verify)
	@$(call gate,_engine-advisories,engine:advisories)

_engine-advisories:
	@echo "== cargo deny advisories =="
	$(CARGO_IN_DOCKER) 'cargo deny --locked check advisories'

## ---- THIRD-PARTY-LICENSES: cargo-about --------------------------------------

# The license texts of the dependencies, written by cargo-about from
# engine/Cargo.lock with engine/about.toml and engine/about.hbs, and committed
# at the repository root. `engine:licenses` fails when the committed file is not
# what cargo-about writes now, so a dependency added or updated without
# `engine:licenses-write` fails that gate. Committed rather than made when a
# package is built, so that the platform builds (macOS, Windows) need no
# cargo-about and a pull request's diff shows the change of licenses.
#
# --workspace lists the dependencies of every workspace crate, the
# xlsx-graft-fixtures-gen tool's (rust_xlsxwriter, zip, ...) included: cargo-about
# has no option to leave one crate out, and a longer list is harmless.
# --offline reads each crate's license files from the cargo cache and never
# fetches one from the crate's repository, so the text does not depend on the
# network; `cargo fetch` downloads the crates into the cargo cache first.
# --fail stops at a crate whose license text cargo-about cannot read.
THIRD_PARTY := THIRD-PARTY-LICENSES
# The two commands are joined with `;`, not `&&`: under `sh -e` a failure of the
# command on the left of `&&` does not stop the shell, so a failed `cargo fetch`
# would go on to check-notice-files.sh and engine:licenses-write would copy an
# incomplete file.
ABOUT_GENERATE := cargo fetch --locked --quiet; \
	cargo about generate --locked --offline --fail --all-features --workspace \
		-c about.toml about.hbs

engine\:licenses: ## THIRD-PARTY-LICENSES is what cargo-about writes from engine/Cargo.lock, with every NOTICE file of the dependencies in it
	@$(call gate,_engine-licenses,engine:licenses)

_engine-licenses:
	@echo "== third-party licenses =="
	$(CARGO_IN_DOCKER) 'out=$$(mktemp); $(ABOUT_GENERATE) -o "$$out"; \
		../scripts/check-notice-files.sh "$$out"; \
		diff -u /repo/$(THIRD_PARTY) "$$out" \
			|| { echo "licenses: $(THIRD_PARTY) is not what cargo-about writes from engine/Cargo.lock; run make engine:licenses-write" >&2; exit 1; }'

engine\:licenses-write: ## Write THIRD-PARTY-LICENSES from engine/Cargo.lock (then: git add THIRD-PARTY-LICENSES)
	@echo "== engine:licenses-write =="
	$(CARGO_IN_DOCKER) 'out=$$(mktemp); $(ABOUT_GENERATE) -o "$$out"; \
		../scripts/check-notice-files.sh "$$out"; \
		cp "$$out" /repo/$(THIRD_PARTY); chmod 644 /repo/$(THIRD_PARTY)'
