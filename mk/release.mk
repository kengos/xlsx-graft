# release — the packages of a release, built from the engine of every platform
#
# Public targets `release:matrix`, `release:native` and `release:assemble`, with
# the recipes in `_release-<job>`, like mk/engine.mk. None is a member of
# `verify`: the engine of the five platforms is built on five runners, which a
# contributor's machine and `make verify` are not, and nothing here publishes.
# .github/workflows/release-artifacts.yml (started by hand, never by a pull
# request) runs them.
#
#   release:matrix     the platforms and their runners, as JSON for the workflow
#   release:native     the C ABI library, the Node.js addon and the CLI of the
#                      Docker host's platform, in the toolchain image
#   release:assemble   the packages from dist/release/native/<slug>/
#
# scripts/release/platforms.tsv is the one list of platforms. The packages that
# carry no engine (the gem and the wheel for every platform) are always built;
# RELEASE_PLATFORMS=<slug>... builds the platform packages of those platforms
# only (a narrowed run, which a PASS line says), for a machine that cannot
# build the others: scripts/release/build-native.sh builds macOS and Windows on
# their own runners.

.PHONY: _release-assemble _release-assemble-dotnet _release-assemble-js _release-assemble-jvm \
        _release-assemble-python _release-assemble-ruby _release-check _release-native \
        _release-smoke \
        release\:assemble release\:matrix release\:native

RELEASE_TABLE := scripts/release/platforms.tsv
RELEASE_ALL := $(shell scripts/release/platform.sh --slugs)
# Command line only (Makefile § knobs).
RELEASE_PLATFORMS :=
RELEASE_SLUGS := $(or $(RELEASE_PLATFORMS),$(RELEASE_ALL))
$(foreach s,$(RELEASE_SLUGS),$(if $(filter $(s),$(RELEASE_ALL)),,$(error RELEASE_PLATFORMS names $(s), which is not in $(RELEASE_TABLE): $(RELEASE_ALL))))

RELEASE_NATIVE := dist/release/native
RELEASE_PACKAGES := dist/release/packages

release\:matrix: ## The platforms of a release and their runners, as JSON for the workflow (no Docker)
	@awk -F'\t' 'BEGIN { printf "{\"include\":[" } \
		/^#/ { next } \
		!header { header = 1; next } \
		{ printf "%s{\"slug\":\"%s\",\"runner\":\"%s\"}", (n++ ? "," : ""), $$1, $$2 } \
		END { print "]}" }' $(RELEASE_TABLE)

release\:native: ## Build the engine of the Docker host's platform (C ABI library, Node.js addon, CLI) into dist/release/native/<slug>/
	@$(call gate,_release-native,release:native)

# The slug is the container's own architecture. The three builds are the
# targets the SDKs' gates use, so each is checked there (the library's symbols,
# the CLI's version).
_release-native: _engine-capi-lib _engine-napi _engine-cli-bin
	@echo "== release native (Docker host platform) =="
	$(CARGO_IN_DOCKER) 'case "$$(uname -m)" in \
			x86_64) slug=linux-x64-gnu ;; \
			aarch64) slug=linux-arm64-gnu ;; \
			*) echo "release:native: no platform for $$(uname -m)" >&2; exit 1 ;; \
		esac; \
		out=/repo/$(RELEASE_NATIVE)/$$slug; rm -rf "$$out"; mkdir -p "$$out"; \
		cp /repo/$(CAPI_LOCAL)/* /repo/$(CLI_LOCAL)/xlsx-graft /repo/$(NAPI_LOCAL)/xlsx_graft.node "$$out/"; \
		echo "native: $(RELEASE_NATIVE)/$$slug"; ls "$$out"'

release\:assemble: ## Build the packages (gems, wheels, CLI archives) from dist/release/native/, check what they hold, load the host's platform (RELEASE_PLATFORMS=<slug>...)
	@$(call gate,_release-assemble,release:assemble)

# Each package is built on the floor line of its language, as docs/sdk.md says a
# package must work on every line; the packages of the Docker host's platform
# are loaded on the latest line. A platform the
# host cannot load (a Mac's Docker is arm64 Linux, so not darwin-x64) is built
# and checked by the names of its files alone, and the smoke test says so.
_release-assemble:
	@echo "== release natives =="
	@missing=; for s in $(RELEASE_SLUGS); do \
		for f in $$(scripts/release/platform.sh $$s capi) $$(scripts/release/platform.sh $$s cli) xlsx_graft.node; do \
			[ -f $(RELEASE_NATIVE)/$$s/$$f ] || missing="$$missing $(RELEASE_NATIVE)/$$s/$$f"; \
		done; \
	done; \
	if [ -n "$$missing" ]; then \
		echo "release:assemble: missing:$$missing" >&2; \
		echo "  make release:native builds the Docker host's platform; the workflow builds the rest" >&2; \
		exit 1; \
	fi
	@$(MAKE) --no-print-directory _release-assemble-ruby RUBY_VER=$(firstword $(RUBY_LINES))
	@$(MAKE) --no-print-directory _release-assemble-python PYTHON_VER=$(firstword $(PYTHON_LINES))
	@$(MAKE) --no-print-directory _release-assemble-dotnet DOTNET_VER=$(firstword $(DOTNET_LINES))
	@$(MAKE) --no-print-directory _release-assemble-jvm JVM_VER=$(firstword $(JVM_LINES))
	@$(MAKE) --no-print-directory _release-assemble-js JS_VER=$(firstword $(JS_LINES))
	@$(MAKE) --no-print-directory _release-check PYTHON_VER=$(firstword $(PYTHON_LINES))
	@$(MAKE) --no-print-directory _release-smoke RUBY_VER=$(lastword $(RUBY_LINES)) \
		PYTHON_VER=$(lastword $(PYTHON_LINES)) DOTNET_VER=$(lastword $(DOTNET_LINES)) \
		JVM_VER=$(lastword $(JVM_LINES)) JS_VER=$(lastword $(JS_LINES))

# Each package builder empties its own directory, so a package left by an
# earlier run never stands in for this one.
_release-assemble-ruby: _sdk-image-ruby
	@echo "== release gems (Ruby $(RUBY_PATCH)) =="
	$(RUBY_IN_DOCKER) 'sh /repo/scripts/release/assemble-ruby.sh $(RELEASE_SLUGS)'

_release-assemble-python: _sdk-image-python
	@echo "== release wheels and CLI archives (Python $(PYTHON_PATCH)) =="
	$(PYTHON_IN_DOCKER) 'sh /repo/scripts/release/assemble-python.sh $(RELEASE_SLUGS); \
		rm -rf /repo/$(RELEASE_PACKAGES)/cli; \
		cli=$$(sed -n "/^\[workspace.package\]/,/^\[/s/^version = \"\(.*\)\"$$/\1/p" /repo/engine/Cargo.toml); \
		python -I /repo/scripts/release/archive_cli.py /repo/$(RELEASE_NATIVE) /repo/$(RELEASE_PACKAGES)/cli "$$cli" $(RELEASE_SLUGS)'

_release-assemble-dotnet: _sdk-image-dotnet
	@echo "== release nupkg (.NET SDK $(DOTNET_PATCH)) =="
	$(DOTNET_IN_DOCKER) 'sh /repo/scripts/release/assemble-dotnet.sh $(RELEASE_SLUGS)'

_release-assemble-jvm: _sdk-image-jvm
	@echo "== release jars (Temurin $(JVM_PATCH)) =="
	$(JVM_IN_DOCKER) 'sh /repo/scripts/release/assemble-jvm.sh $(RELEASE_SLUGS)'

_release-assemble-js: _sdk-image-js
	@echo "== release npm packages (Node.js $(JS_PATCH)) =="
	$(JS_IN_DOCKER) 'sh /repo/scripts/release/assemble-js.sh $(RELEASE_SLUGS)'

# What each package holds (scripts/release/check_packages.py), then one
# checksum list over every package.
_release-check: _sdk-image-python
	@echo "== release packages (names of the files in each) =="
	$(PYTHON_IN_DOCKER) 'python -I /repo/scripts/release/check_packages.py /repo/$(RELEASE_PACKAGES) $(RELEASE_SLUGS); \
		cd /repo/$(RELEASE_PACKAGES) && rm -f SHA256SUMS \
			&& find . -type f | sort | xargs sha256sum | sed "s| \./| |" > /tmp/SHA256SUMS \
			&& cp /tmp/SHA256SUMS SHA256SUMS && cat SHA256SUMS'

# Installs the packages of the Docker host's platform, a Linux one, with no
# XLSX_GRAFT_LIBRARY, so each package reaches the engine it carries and no other.
_release-smoke: _sdk-image-ruby _sdk-image-python _sdk-image-dotnet _sdk-image-jvm _sdk-image-js
	@host=$$(docker info --format '{{.Architecture}}'); \
	case "$$host" in \
		x86_64) slug=linux-x64-gnu ;; \
		aarch64|arm64) slug=linux-arm64-gnu ;; \
		*) slug= ;; \
	esac; \
	want=$$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"$$/\1/p' engine/Cargo.toml); \
	case " $(RELEASE_SLUGS) " in \
		*" $$slug "*) \
			echo "== release smoke: $$slug, Ruby $(RUBY_PATCH) =="; \
			$(RUBY_IN_DOCKER) "sh /repo/scripts/release/smoke-ruby.sh $$slug $$want" || exit 1; \
			echo "== release smoke: $$slug, Python $(PYTHON_PATCH) =="; \
			$(PYTHON_IN_DOCKER) "sh /repo/scripts/release/smoke-python.sh $$slug $$want" || exit 1; \
			echo "== release smoke: $$slug, .NET SDK $(DOTNET_PATCH) =="; \
			$(DOTNET_IN_DOCKER) "sh /repo/scripts/release/smoke-dotnet.sh" || exit 1; \
			echo "== release smoke: $$slug, Temurin $(JVM_PATCH) =="; \
			$(JVM_IN_DOCKER) "sh /repo/scripts/release/smoke-jvm.sh $$slug" || exit 1; \
			echo "== release smoke: $$slug, Node.js $(JS_PATCH) =="; \
			$(JS_IN_DOCKER) "sh /repo/scripts/release/smoke-js.sh $$slug $$want" || exit 1 ;; \
		*) echo "== release smoke: skipped, the Docker host ($$host) is not one of: $(RELEASE_SLUGS) ==" ;; \
	esac
