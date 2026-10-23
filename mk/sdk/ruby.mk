# sdk/ruby — the Ruby SDK's gates and lockfile (mk/sdk.mk names the values a
# language's file sets and what `sdk_lang` makes of them)

.PHONY: _sdk-image-ruby _sdk-lint-ruby _sdk-lint-ruby-line _sdk-test-ruby _sdk-test-ruby-line \
        _sdk-verify-ruby _sdk-verify-ruby-line sdk\:lint\:ruby sdk\:lock\:ruby sdk\:test\:ruby \
        sdk\:verify\:ruby

# The floor first and the latest stable line last: LATEST=1 takes the last and
# sdk:lock:ruby the first. The base image's Debian layers still move when
# Docker Hub rebuilds the tag a patch release names.
RUBY_LINES := 3.4 4.0
RUBY_PATCH_3.4 := 3.4.11
RUBY_PATCH_4.0 := 4.0.7
# Command line only (Makefile § knobs).
RUBY_VER :=
RUBY_IMAGE_INPUTS := $(addprefix sdk/ruby/,Dockerfile Gemfile Gemfile.lock xlsx_graft.gemspec \
                       lib/xlsx_graft/version.rb)
$(eval $(call sdk_lang,ruby,RUBY))

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
