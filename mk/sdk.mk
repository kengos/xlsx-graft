# sdk/ — the language SDKs: gates and lockfiles
#
# Every target defined here is named `sdk:<job>:<lang>` (public) or
# `_sdk-<job>-<lang>` (private, carrying the recipe). docs/sdk.md is the shape
# every language follows.

.PHONY: _sdk-image-ruby _sdk-lint-ruby _sdk-test-ruby _sdk-verify-ruby \
        sdk\:lint\:ruby sdk\:lock\:ruby sdk\:test\:ruby sdk\:verify\:ruby

## ---- ruby --------------------------------------------------------------

# RUBY_VER picks the release line, and the line its patch release, so the same
# commit gates on the same interpreter. The base image's Debian layers still
# move when Docker Hub rebuilds that tag. 3.4 is the floor (docs/sdk.md
# § Supported language versions).
RUBY_VER ?= 3.4
RUBY_PATCH_3.4 := 3.4.11
RUBY_PATCH_4.0 := 4.0.7
RUBY_PATCH = $(or $(RUBY_PATCH_$(RUBY_VER)),$(error RUBY_VER=$(RUBY_VER) is not a gated line; use 3.4 or 4.0))

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
# CI's and an Apple Silicon Mac's Docker.
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
sdk\:verify\:ruby: ## Verify sdk/ruby — rubocop + rspec at 100% line coverage + gem build/install/load (RUBY_VER=3.4|4.0)
	@$(call gate,_sdk-verify-ruby,sdk:verify:ruby)

_sdk-verify-ruby: _engine-capi-lib _sdk-image-ruby
	@echo "== sdk ruby lint + spec =="
	$(RUBY_IN_DOCKER) 'bundle exec rake lint spec'
	@echo "== sdk ruby gem (build, install, load) =="
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

sdk\:test\:ruby: ## sdk/ruby rspec at 100% line coverage (RUBY_VER=3.4|4.0)
	@$(call gate,_sdk-test-ruby,sdk:test:ruby)

_sdk-test-ruby: _engine-capi-lib _sdk-image-ruby
	@echo "== sdk ruby spec =="
	$(RUBY_IN_DOCKER) 'bundle exec rake spec'

# No engine library needed, so no capi-lib: lint runs without a Rust build.
sdk\:lint\:ruby: ## sdk/ruby rubocop (RUBY_VER=3.4|4.0)
	@$(call gate,_sdk-lint-ruby,sdk:lint:ruby)

_sdk-lint-ruby: _sdk-image-ruby
	@echo "== sdk ruby lint =="
	$(RUBY_IN_DOCKER) 'bundle exec rake lint'
