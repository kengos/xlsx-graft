# sdk/ — the language SDKs: gates and lockfiles
#
# The public targets defined here are named `sdk:<job>` (every language) and
# `sdk:<job>:<lang>`; the private ones, carrying the recipes,
# `_sdk-<job>[-<lang>[-line]]`, the `-line` one running a single release
# line. docs/sdk.md is the shape every language follows.
#
# A gate runs every release line a language supports, one after another;
# LATEST=1 runs the latest stable line only (docs/sdk.md § Supported language
# versions).

.PHONY: _sdk-image-ruby _sdk-lint _sdk-lint-ruby _sdk-lint-ruby-line _sdk-test _sdk-test-ruby \
        _sdk-test-ruby-line _sdk-verify _sdk-verify-ruby _sdk-verify-ruby-line sdk\:lint \
        sdk\:lint\:ruby sdk\:lock\:ruby sdk\:test sdk\:test\:ruby sdk\:verify sdk\:verify\:ruby

## ---- every language ----------------------------------------------------

# Every SDK, by the <lang> of its targets. A language joins here, which puts it
# in sdk:verify / test / lint and, through its CI_JOBS_<lang>, in CI.
SDK_LANGS := ruby

sdk\:verify: ## Verify every SDK on every supported line (LATEST=1: the latest line only)
	@$(call gate,_sdk-verify,sdk:verify)

_sdk-verify: $(addprefix _sdk-verify-,$(SDK_LANGS))

sdk\:test: ## Every SDK's tests at 100% line coverage, on every supported line (LATEST=1: the latest)
	@$(call gate,_sdk-test,sdk:test)

_sdk-test: $(addprefix _sdk-test-,$(SDK_LANGS))

sdk\:lint: ## Every SDK's lint, on every supported line (LATEST=1: the latest)
	@$(call gate,_sdk-lint,sdk:lint)

_sdk-lint: $(addprefix _sdk-lint-,$(SDK_LANGS))

# One CI job per language and per line, in the shape `ci:matrix` prints
# (Makefile).
SDK_CI_JOBS = $(foreach l,$(SDK_LANGS),$(or $(CI_JOBS_$(l)),$(error SDK_LANGS lists $(l), but no CI_JOBS_$(l) lists its CI jobs)))

## ---- ruby --------------------------------------------------------------

# RUBY_LINES lists the supported release lines, the floor first and the
# latest stable line last, and each line is pinned to a patch release, so the
# same commit gates on the same interpreter. The base image's Debian layers
# still move when Docker Hub rebuilds that tag. A gate runs every line,
# LATEST=1 the last one, and RUBY_VER=<line> that line alone.
RUBY_LINES := 3.4 4.0
RUBY_PATCH_3.4 := 3.4.11
RUBY_PATCH_4.0 := 4.0.7
# Command line only (Makefile § knobs).
RUBY_VER :=
RUBY_GATED = $(or $(RUBY_VER),$(if $(LATEST),$(lastword $(RUBY_LINES)),$(RUBY_LINES)))
RUBY_PATCH = $(or $(RUBY_PATCH_$(RUBY_VER)),$(error RUBY_VER is unset; run a public sdk:*:ruby target))

# Each entry opens with the comma that joins it to the one before (CI_MATRIX,
# Makefile).
CI_JOBS_ruby = $(foreach v,$(RUBY_LINES),,{"target":"sdk:verify:ruby","args":"RUBY_VER=$(v)"})

# Refused when make reads the file, before any gate builds the engine.
ifneq ($(filter-out $(RUBY_LINES),$(RUBY_VER)),)
$(error RUBY_VER=$(RUBY_VER) is not a gated line; use one of $(RUBY_LINES))
endif

# Each line runs in a make of its own: RUBY_VER fixes the image every recipe
# below names, and one make runs a target once. The first red line stops the
# rest.
ruby_each = for v in $(RUBY_GATED); do $(MAKE) --no-print-directory $(1) RUBY_VER=$$v || exit; done

# Built from sdk/ruby/Dockerfile on every gate and never published: it holds
# the bundle alone, and docker's layer cache keeps a rebuild to a no-op until
# Gemfile.lock changes. The tag carries a hash of the files the image is built
# from, because gates of two working trees may run at once: under one shared
# tag, a tree whose Gemfile.lock differs would run on the other tree's bundle.
RUBY_IMAGE_INPUTS := $(addprefix sdk/ruby/,Dockerfile Gemfile Gemfile.lock xlsx_graft.gemspec \
                       lib/xlsx_graft/version.rb)
RUBY_IMAGE_HASH := $(shell cat $(RUBY_IMAGE_INPUTS) | git hash-object --stdin | cut -c1-12)
RUBY_IMAGE = xlsx-graft-sdk-ruby:$(RUBY_PATCH)-$(RUBY_IMAGE_HASH)

# The engine library comes from the mount, not the image (sdk/ruby/Dockerfile
# says why). Inside Docker it is always the Linux .so. The same container
# label and --init as CARGO_IN_DOCKER; --pull never because the image exists
# only locally, and a missing one is a local error rather than a failed pull.
RUBY_IN_DOCKER = $(GATE_LOCK) docker run --rm --init --pull never \
	--label "xlsx-graft.tree=$(CURDIR)" \
	-v "$(CURDIR):/repo" -w /repo/sdk/ruby \
	-e XLSX_GRAFT_LIBRARY=/repo/$(CAPI_LOCAL)/libxlsx_graft_capi.so \
	$(RUBY_IMAGE) sh -euc

# Writes sdk/ruby/Gemfile.lock and does NOT go through `quiet`, like
# engine:lock. It runs on the plain ruby image: the gate image installs from
# the lockfile, so it cannot be the one that makes it. The two platforms are
# CI's and an Apple Silicon Mac's Docker. It resolves on the floor, so bundler
# never picks a gem that the floor cannot install; whether every later line
# installs it is what that line's gate checks.
sdk\:lock\:ruby: override RUBY_VER = $(firstword $(RUBY_LINES))
sdk\:lock\:ruby: ## Re-resolve sdk/ruby/Gemfile.lock after a Gemfile or gemspec edit (then: git add sdk/ruby/Gemfile.lock)
	@echo "== sdk:lock:ruby (Ruby $(RUBY_PATCH)) =="
	$(GATE_LOCK) docker run --rm --init --label "xlsx-graft.tree=$(CURDIR)" \
		-v "$(CURDIR):/repo" -w /repo/sdk/ruby ruby:$(RUBY_PATCH)-slim-bookworm \
		bundle lock --add-platform x86_64-linux aarch64-linux

_sdk-image-ruby:
	@echo "== sdk ruby image (Ruby $(RUBY_PATCH)) =="
	@docker build -q --build-arg RUBY_VERSION=$(RUBY_PATCH) -t $(RUBY_IMAGE) sdk/ruby >/dev/null

# Lint, the specs at 100% line coverage, then the gem as a user gets it: built
# from a copy of the checkout (minus any library left in lib/xlsx_graft/native/,
# which the gem would otherwise ship), installed into a directory of its own (GEM_HOME;
# fiddle still comes from the image's gems on GEM_PATH, as --local fetches
# nothing), and loaded from there with no bundler and no checkout on the load
# path.
sdk\:verify\:ruby: ## Verify sdk/ruby on every supported line — rubocop + rspec at 100% line coverage + gem build/install/load (LATEST=1, RUBY_VER=<line>)
	@$(call gate,_sdk-verify-ruby,sdk:verify:ruby)

_sdk-verify-ruby: _engine-capi-lib
	@$(call ruby_each,_sdk-verify-ruby-line)

_sdk-verify-ruby-line: _sdk-image-ruby
	@echo "== sdk ruby lint + spec (Ruby $(RUBY_PATCH)) =="
	$(RUBY_IN_DOCKER) 'bundle exec rake lint spec'
	@echo "== sdk ruby gem (build, install, load) (Ruby $(RUBY_PATCH)) =="
	$(RUBY_IN_DOCKER) 'cp -r /repo/sdk/ruby /tmp/build; \
		rm -rf /tmp/build/coverage /tmp/build/lib/xlsx_graft/native; \
		cd /tmp/build; \
		gem build xlsx_graft.gemspec; \
		unset BUNDLE_GEMFILE; \
		gem_path=$$(gem env gempath); \
		export GEM_PATH="/tmp/gems:$$gem_path"; \
		export GEM_HOME=/tmp/gems; \
		gem install --local --no-document xlsx_graft-*.gem; \
		cd /tmp; \
		ruby /repo/sdk/ruby/script/check_installed_gem.rb /tmp/gems /repo/engine/Cargo.toml'

sdk\:test\:ruby: ## sdk/ruby rspec at 100% line coverage, on every supported line (LATEST=1, RUBY_VER=<line>)
	@$(call gate,_sdk-test-ruby,sdk:test:ruby)

_sdk-test-ruby: _engine-capi-lib
	@$(call ruby_each,_sdk-test-ruby-line)

_sdk-test-ruby-line: _sdk-image-ruby
	@echo "== sdk ruby spec (Ruby $(RUBY_PATCH)) =="
	$(RUBY_IN_DOCKER) 'bundle exec rake spec'

# No engine library needed, so no capi-lib: lint runs without a Rust build.
sdk\:lint\:ruby: ## sdk/ruby rubocop, on every supported line (LATEST=1, RUBY_VER=<line>)
	@$(call gate,_sdk-lint-ruby,sdk:lint:ruby)

_sdk-lint-ruby:
	@$(call ruby_each,_sdk-lint-ruby-line)

_sdk-lint-ruby-line: _sdk-image-ruby
	@echo "== sdk ruby lint (Ruby $(RUBY_PATCH)) =="
	$(RUBY_IN_DOCKER) 'bundle exec rake lint'
