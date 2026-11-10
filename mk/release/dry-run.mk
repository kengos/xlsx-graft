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
#   version:check               the versions of the crates' path dependencies
#                               and of the Ruby, Python, .NET, Java and JS
#                               manifests are the workspace's
#                               (scripts/check-versions.sh; no Docker)
#   release:dry-run:crates      `cargo publish --dry-run` of the crates on
#                               crates.io (the CLI, the MCP server and the
#                               engine crates they build on; capi and napi go in
#                               the SDK packages), then what each .crate holds
#   release:dry-run:rubygems    the gems pass the validation rubygems.org runs
#   release:dry-run:pypi        `twine check --strict` of the wheels
#   release:dry-run:nuget       the nupkg's nuspec has what nuget.org requires
#   release:dry-run:maven       the Maven Central bundle (signed with a key
#                               generated for the run) and the POM's required
#                               fields
#   release:dry-run:npm         `npm publish --dry-run` of each npm package
#   release:dry-run:packagist   `composer validate --strict` and the archive
#   release:dry-run:go          the zip the Go proxy makes of the module
#   release:dry-run             every registry's dry run, and version:check
#
# The registries of the packages that carry the engine (RubyGems, PyPI, NuGet,
# Maven Central, npm) check the packages release:assemble builds for the Docker
# host's platform (RELEASE_HOST), which release:native builds first. The
# packages of the other platforms differ from the host's only in the engine file
# they hold; check_packages.py checks the names of the engine files in the
# release workflow. Each registry's check runs on the latest line of its
# language (the packages are built on the floor line).
#
# A .crate, the Composer archive and the Go module's zip hold source alone, so
# each holds the license files (LICENSE-APACHE, LICENSE-BSD, LICENSE-MIT,
# NOTICE) and no THIRD-PARTY-LICENSES: the licenses of the dependencies are in
# the packages that hold the engine binary.

.PHONY: _release-dry-run _release-dry-run-crates _release-dry-run-crates-check \
        _release-dry-run-go _release-dry-run-go-check _release-dry-run-go-zip _release-dry-run-maven \
        _release-dry-run-maven-bundle _release-dry-run-maven-check _release-dry-run-npm \
        _release-dry-run-npm-run _release-dry-run-nuget _release-dry-run-nuget-check \
        _release-dry-run-packages _release-dry-run-packagist _release-dry-run-packagist-archive \
        _release-dry-run-packagist-check _release-dry-run-pypi _release-dry-run-pypi-run \
        _release-dry-run-rubygems _release-dry-run-rubygems-run _version-check \
        release\:dry-run release\:dry-run\:crates release\:dry-run\:go \
        release\:dry-run\:maven release\:dry-run\:npm release\:dry-run\:nuget \
        release\:dry-run\:packagist release\:dry-run\:pypi release\:dry-run\:rubygems \
        version\:check

RELEASE_DRY_RUN := dist/release/dry-run
CRATES_NOT_PUBLISHED := --exclude xlsx-graft-capi --exclude xlsx-graft-napi --exclude xlsx-graft-fixtures-gen
# [workspace.package] version, which a Composer archive and a Go module take
# as the tag's.
WORKSPACE_VERSION := $(shell sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"$$/\1/p' engine/Cargo.toml)
$(if $(WORKSPACE_VERSION),,$(error no [workspace.package] version in engine/Cargo.toml))

version\:check: ## Fail when a package manifest's version is not the workspace's (no Docker)
	@$(call gate,_version-check,version:check)

_version-check:
	@echo "== version check =="
	@scripts/check-versions.sh

release\:dry-run: ## Every registry's dry run and version:check, nothing published (a member of verify)
	@$(call gate,_release-dry-run,release:dry-run)

_release-dry-run: _version-check _release-dry-run-crates _release-dry-run-rubygems \
	_release-dry-run-pypi _release-dry-run-nuget _release-dry-run-maven _release-dry-run-npm \
	_release-dry-run-packagist _release-dry-run-go

# The packages of the Docker host's platform, built from its engine. A host that
# is none of the release's platforms cannot make them.
_release-dry-run-packages: _release-native
	@[ -n "$(RELEASE_HOST)" ] || { echo "release:dry-run: the Docker host is none of the release's platforms ($(RELEASE_ALL))" >&2; exit 1; }
	@$(MAKE) --no-print-directory _release-assemble RELEASE_PLATFORMS=$(RELEASE_HOST)

## ---- crates.io -------------------------------------------------------------

release\:dry-run\:crates: ## cargo publish --dry-run of the crates published to crates.io, and the files each .crate holds
	@$(call gate,_release-dry-run-crates,release:dry-run:crates)

# --allow-dirty: a contributor's tree is rarely clean; the release workflow
# publishes a tag's checkout without it. --workspace skips a crate that says
# `publish = false`. `cargo publish --dry-run` builds each .crate to verify it
# and does not keep it, so `cargo package --no-verify` writes the .crate files
# that check_packages.py reads (--no-verify: the dry run has compiled them).
# Unlike publish, `cargo package --workspace` does not skip a crate that says
# `publish = false`, so the three are excluded by name; the check then expects
# exactly the crates that are published, so a crate left out here or added
# without its `publish = false` fails it.
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

## ---- RubyGems --------------------------------------------------------------

release\:dry-run\:rubygems: ## Validate the gems the way rubygems.org does (Gem::Specification#validate, strict)
	@$(call gate,_release-dry-run-rubygems,release:dry-run:rubygems)

_release-dry-run-rubygems: _release-dry-run-packages
	@$(MAKE) --no-print-directory _release-dry-run-rubygems-run RUBY_VER=$(lastword $(RUBY_LINES))

_release-dry-run-rubygems-run: _sdk-image-ruby
	@echo "== release dry run: RubyGems (Ruby $(RUBY_PATCH)) =="
	$(RUBY_IN_DOCKER) 'ruby /repo/scripts/release/dry-run/check_gems.rb /repo /repo/$(RELEASE_PACKAGES)/ruby'

## ---- PyPI ------------------------------------------------------------------

release\:dry-run\:pypi: ## twine check --strict of the wheels
	@$(call gate,_release-dry-run-pypi,release:dry-run:pypi)

_release-dry-run-pypi: _release-dry-run-packages
	@$(MAKE) --no-print-directory _release-dry-run-pypi-run PYTHON_VER=$(lastword $(PYTHON_LINES))

_release-dry-run-pypi-run: _sdk-image-python
	@echo "== release dry run: PyPI (Python $(PYTHON_PATCH)) =="
	$(PYTHON_IN_DOCKER) 'twine check --strict /repo/$(RELEASE_PACKAGES)/python/*.whl'

## ---- NuGet -----------------------------------------------------------------

release\:dry-run\:nuget: ## The nupkg's nuspec has what nuget.org requires (license expression, repository, readme)
	@$(call gate,_release-dry-run-nuget,release:dry-run:nuget)

_release-dry-run-nuget: _release-dry-run-packages
	@$(MAKE) --no-print-directory _release-dry-run-nuget-check PYTHON_VER=$(lastword $(PYTHON_LINES))

_release-dry-run-nuget-check: _sdk-image-python
	@echo "== release dry run: NuGet =="
	$(PYTHON_IN_DOCKER) 'python -I /repo/scripts/release/dry-run/check_nuspec.py /repo /repo/$(RELEASE_PACKAGES)/dotnet/*.nupkg'

## ---- Maven Central ---------------------------------------------------------

release\:dry-run\:maven: ## The Maven Central bundle, signed with a key generated for the run, and the POM's required fields
	@$(call gate,_release-dry-run-maven,release:dry-run:maven)

_release-dry-run-maven: _release-dry-run-packages
	@$(MAKE) --no-print-directory _release-dry-run-maven-bundle JVM_VER=$(lastword $(JVM_LINES))
	@$(MAKE) --no-print-directory _release-dry-run-maven-check PYTHON_VER=$(lastword $(PYTHON_LINES))

# GNUPGHOME is a temporary directory inside the container: the key is deleted
# when the container exits.
_release-dry-run-maven-bundle: _sdk-image-jvm
	@echo "== release dry run: Maven Central bundle (Temurin $(JVM_PATCH)) =="
	$(JVM_IN_DOCKER) 'GNUPGHOME=$$(mktemp -d); export GNUPGHOME; \
		gpg --batch --quiet --pinentry-mode loopback --passphrase "" \
			--quick-generate-key "xlsx-graft dry run <dry-run@invalid>" ed25519 sign never; \
		rm -rf /repo/$(RELEASE_DRY_RUN)/maven; \
		sh /repo/scripts/release/maven-bundle.sh /repo/$(RELEASE_PACKAGES)/jvm /repo/$(RELEASE_DRY_RUN)/maven/bundle.zip'

_release-dry-run-maven-check: _sdk-image-python
	@echo "== release dry run: what the bundle holds =="
	$(PYTHON_IN_DOCKER) 'python -I /repo/scripts/release/dry-run/check_maven.py /repo/$(RELEASE_DRY_RUN)/maven/bundle.zip $(shell scripts/release/platform.sh $(RELEASE_HOST) jna)'

## ---- npm -------------------------------------------------------------------

release\:dry-run\:npm: ## npm publish --dry-run of the package of the classes and of the platform packages
	@$(call gate,_release-dry-run-npm,release:dry-run:npm)

_release-dry-run-npm: _release-dry-run-packages
	@$(MAKE) --no-print-directory _release-dry-run-npm-run JS_VER=$(lastword $(JS_LINES))

_release-dry-run-npm-run: _sdk-image-js
	@echo "== release dry run: npm (Node.js $(JS_PATCH)) =="
	$(JS_IN_DOCKER) 'sh /repo/scripts/release/dry-run/npm.sh /repo/$(RELEASE_PACKAGES)/js'

## ---- Packagist -------------------------------------------------------------

release\:dry-run\:packagist: ## composer validate --strict, and the archive of sdk/php holds the license files
	@$(call gate,_release-dry-run-packagist,release:dry-run:packagist)

# Packagist reads the composer.json at the root of a repository, so sdk/php is
# split into a repository of its own at a release, with the license files
# beside it: the copies in sdk/php, which must equal the files at the root of
# this repository.
_release-dry-run-packagist:
	@echo "== release dry run: Packagist =="
	@scripts/check-license-copies.sh sdk/php
	@$(MAKE) --no-print-directory _release-dry-run-packagist-archive PHP_VER=$(lastword $(PHP_LINES))
	@$(MAKE) --no-print-directory _release-dry-run-packagist-check PYTHON_VER=$(lastword $(PYTHON_LINES))

_release-dry-run-packagist-archive: _sdk-image-php
	@echo "== release dry run: composer validate and archive (PHP $(PHP_PATCH)) =="
	$(PHP_IN_DOCKER) 'rm -rf /repo/$(RELEASE_DRY_RUN)/packagist; \
		cp -r /repo/sdk/php /tmp/build; \
		cd /tmp/build; \
		rm -rf vendor build .phpunit.cache .php-cs-fixer.cache; \
		$(PHP_VALIDATE); \
		composer config version "$(WORKSPACE_VERSION)"; \
		composer archive --format=zip --dir=/repo/$(RELEASE_DRY_RUN)/packagist --file=xlsx-graft'

_release-dry-run-packagist-check: _sdk-image-python
	@echo "== release dry run: what the archive holds =="
	$(PYTHON_IN_DOCKER) 'python -I /repo/scripts/release/check_packages.py --archive /repo/$(RELEASE_DRY_RUN)/packagist/xlsx-graft.zip'

## ---- Go --------------------------------------------------------------------

release\:dry-run\:go: ## The zip the Go proxy makes of sdk/go (golang.org/x/mod/zip) holds the license files
	@$(call gate,_release-dry-run-go,release:dry-run:go)

# The zip is made with golang.org/x/mod/zip, the code the proxy runs, which
# refuses a symbolic link and a file name Go modules do not take. The module's
# license files must be byte-for-byte equal to the files at the root of this
# repository (a module zip cannot hold symbolic links).
_release-dry-run-go:
	@echo "== release dry run: Go =="
	@scripts/check-license-copies.sh sdk/go
	@$(MAKE) --no-print-directory _release-dry-run-go-zip GO_VER=$(lastword $(GO_LINES))
	@$(MAKE) --no-print-directory _release-dry-run-go-check PYTHON_VER=$(lastword $(PYTHON_LINES))

_release-dry-run-go-zip: _sdk-image-go
	@echo "== release dry run: the module's zip (Go $(GO_PATCH)) =="
	$(GO_IN_DOCKER) 'rm -rf /repo/$(RELEASE_DRY_RUN)/go; \
		mkdir -p /repo/$(RELEASE_DRY_RUN)/go; \
		$(GO_MODULE_COPY); \
		module=$$(sed -n "s/^module //p" /tmp/build/go.mod); \
		modzip /tmp/build "$$module" "v$(WORKSPACE_VERSION)" /repo/$(RELEASE_DRY_RUN)/go/module.zip'

_release-dry-run-go-check: _sdk-image-python
	@echo "== release dry run: what the module's zip holds =="
	$(PYTHON_IN_DOCKER) 'python -I /repo/scripts/release/check_packages.py --archive /repo/$(RELEASE_DRY_RUN)/go/module.zip'
