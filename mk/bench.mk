# bench — comparing xlsx-graft with the Ruby xlsx gems (bench/ruby)
#
# Not a gate: the result changes with the machine and the day, not with the
# commit. None of these targets prints PASS/FAIL, none is a member of `verify`
# or of `ci:matrix`, and each prints the raw output of what it ran.
#
# The scripts run in an image built from bench/ruby/Dockerfile, never
# published, on the latest Ruby line the SDK gates (RUBY_LINES). The container
# gets 8 GiB of memory and no swap, so a reader that needs more is killed and
# reported as such. The tag carries a hash of the files the image is built from
# (as in mk/sdk.mk), because the benchmarks of two working trees may run at once.
#
# Every target holds the gate lock (GATE_LOCK) while it runs, which can be hours
# on bench:compare: a `make engine:verify` of the same working tree waits for it.
# The lock is needed because _engine-capi-lib deletes dist/capi/local.
#
# Both bench targets run fixtures:generate first, which writes the books again,
# so that the books measured are the ones the checked-out fixtures-gen writes.
#
# BENCH_COMMIT and BENCH_HOST are passed in as environment variables and
# printed in the report's conditions: the commit (with `-dirty` when the tree
# has changes) and the machine (scripts/bench-host.sh).

.PHONY: _bench-image bench\:cli bench\:compare bench\:lock

BENCH_RUBY_PATCH := $(RUBY_PATCH_$(lastword $(RUBY_LINES)))
BENCH_IMAGE_INPUTS := $(addprefix bench/ruby/,Dockerfile Gemfile Gemfile.lock)
BENCH_IMAGE_HASH := $(shell cat $(BENCH_IMAGE_INPUTS) 2>/dev/null | git hash-object --stdin | cut -c1-12)
BENCH_IMAGE := xlsx-graft-bench-ruby:$(BENCH_RUBY_PATCH)-$(BENCH_IMAGE_HASH)

# The same container label and --init as CARGO_IN_DOCKER; --pull never because
# the image exists only locally. The repository is mounted at /repo, which is
# also the working directory, so the scripts' paths are relative to it.
BENCH_IN_DOCKER = $(GATE_LOCK) docker run --rm --init --pull never \
	--memory 8g --memory-swap 8g \
	--label "xlsx-graft.tree=$(CURDIR)" \
	-v "$(CURDIR):/repo" -w /repo \
	-e XLSX_GRAFT_LIBRARY=/repo/$(CAPI_LOCAL)/libxlsx_graft_capi.so \
	-e XLSX_GRAFT_CLI=/repo/$(CLI_LOCAL)/xlsx-graft \
	-e "BENCH_COMMIT=$$(git rev-parse --short HEAD)$$(test -z "$$(git status --porcelain)" || echo -dirty)" \
	-e "BENCH_HOST=$$(scripts/bench-host.sh)" \
	$(BENCH_IMAGE)

# Writes bench/ruby/Gemfile.lock and does NOT go through `quiet`, like
# sdk:lock:ruby, and for the same reason runs on the plain ruby image: the
# bench image installs from the lockfile, so it cannot be the one that makes it.
# --add-platform adds two platforms, CI's and an Apple Silicon Mac's Docker;
# the others in the lockfile are added by Bundler, to match nokogiri's
# precompiled gems.
bench\:lock: ## Re-resolve bench/ruby/Gemfile.lock after a Gemfile edit (then: git add bench/ruby/Gemfile.lock)
	@echo "== bench:lock (Ruby $(BENCH_RUBY_PATCH)) =="
	$(GATE_LOCK) docker run --rm --init --label "xlsx-graft.tree=$(CURDIR)" \
		-v "$(CURDIR):/repo" -w /repo/bench/ruby ruby:$(BENCH_RUBY_PATCH)-slim-bookworm \
		bundle lock --add-platform x86_64-linux aarch64-linux

_bench-image:
	@echo "== bench image (Ruby $(BENCH_RUBY_PATCH)) =="
	@docker build -q --label "xlsx-graft.tree=$(CURDIR)" --build-arg RUBY_VERSION=$(BENCH_RUBY_PATCH) \
		-t $(BENCH_IMAGE) bench/ruby >/dev/null

# Every book of Plan::BOOKS (the four `make fixtures:generate` writes) with
# every Ruby reader and the CLI, three runs each: peak memory and time, with the cost of loading the reader
# subtracted. Takes hours when a reader needs the 30 minutes it is allowed on a
# book of 1,000,000 rows.
bench\:compare: fixtures\:generate _engine-capi-lib _engine-cli-bin _bench-image ## Compare xlsx-graft with rubyXL, Roo, Creek, Xsv and simple_xlsx_reader on the generated workbooks; writes dist/bench/compare.json and .md (not in verify)
	@echo "== bench:compare =="
	$(BENCH_IN_DOCKER) bundle exec ruby bench/ruby/compare.rb --out dist/bench/compare

# The CLI alone, for a machine with no Ruby comparison to make (CI's runner).
bench\:cli: fixtures\:generate _engine-cli-bin _bench-image ## Measure the CLI's read and verify on the generated workbooks; writes dist/bench/cli.json and .md (not in verify)
	@echo "== bench:cli =="
	$(BENCH_IN_DOCKER) bundle exec ruby bench/ruby/compare.rb --only cli --out dist/bench/cli
