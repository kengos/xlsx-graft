# xlsx-graft local gates.
#
# THESE ARE THE ONLY SANCTIONED WAYS TO CHECK ANYTHING. Do not invent an
# equivalent (a hand-typed `docker run … cargo test`); if what you need is
# missing, add a target here.   Full inventory: make help
#
# Every name is <scope>:<job>, outside-in. Each job has exactly ONE name; add
# V=1 to any of them for the raw output instead of the PASS/FAIL line.
#
#   Is it correct?            one PASS/FAIL line, non-zero exit on FAIL, log kept
#     make verify               the whole bar: every scope's verify (today
#                               engine:verify and sdk:verify), every SDK on
#                               every line it supports
#     make verify LATEST=1      the same, each SDK on its latest line only
#     make engine:verify        engine: budget + toolchain pins + fixtures/
#                               (README rows, only workbooks, the README
#                               and AppleScripts and no links, allowed
#                               parts, account fields and HyperlinkBase
#                               empty or absent, no absPath,
#                               alternateUrls, refreshedBy, fileSharing,
#                               securityDescriptor or home folder paths,
#                               external targets file names, Office
#                               Script list placeholder, VBA project only
#                               macro.xlsm's reviewed one, the one part
#                               allowed a home folder path) + cargo deny
#                               (licenses, bans, sources) + the committed
#                               THIRD-PARTY-LICENSES is what cargo-about
#                               writes, NOTICE files of the dependencies
#                               included + fmt + clippy + tests at 100% line
#                               coverage + doctests + C ABI symbols, in the
#                               pinned toolchain image
#     make sdk:verify           every SDK, every line it supports (LATEST=1)
#     make sdk:verify:ruby      the Ruby SDK on every supported line (LATEST=1,
#                               or RUBY_VER=<line> for one): rubocop + rspec at
#                               100% line coverage + the gem built, installed
#                               and loaded
#     make sdk:verify:python    the Python SDK on every supported line (LATEST=1,
#                               or PYTHON_VER=<line> for one): ruff + mypy
#                               --strict + pytest at 100% line coverage + the
#                               wheel built, installed and loaded
#     make sdk:verify:dotnet    the .NET SDK on every supported line (LATEST=1,
#                               or DOTNET_VER=<line> for one): dotnet format +
#                               analyzers + xUnit at 100% line coverage + the
#                               nupkg packed, installed and loaded
#     make sdk:verify:jvm       the Java SDK on every supported line (LATEST=1,
#                               or JVM_VER=<line> for one): Spotless + javac
#                               lint + javadoc + JUnit at 100% line coverage +
#                               the jar published, installed and loaded, both
#                               on the line's JVM
#     make sdk:verify:js        the JS SDK on every supported line (LATEST=1, or
#                               JS_VER=<line> for one): Biome + tsc + Vitest at
#                               100% line coverage + the npm package packed,
#                               installed and loaded, against the Node.js addon
#     make sdk:verify:php       the PHP SDK on every supported line (LATEST=1,
#                               or PHP_VER=<line> for one): php-cs-fixer +
#                               PHPStan + PHPUnit at 100% line coverage + the
#                               package archived, installed and loaded, against
#                               the CLI
#     make sdk:verify:go        the Go SDK on every supported line (LATEST=1, or
#                               GO_VER=<line> for one): gofmt + go vet +
#                               golangci-lint + go test -race at 100% statement
#                               coverage + the module copied, built and run,
#                               against the CLI
#
#   Faster slices while iterating
#     make engine:budget        make engine:fmt         make engine:clippy
#     make engine:lint          make engine:test [P=<crate>] [F=<filter>]
#     make engine:coverage      make engine:capi-abi
#     make engine:deny          make engine:licenses
#     make sdk:lint[:<lang>]    make sdk:test[:<lang>]  (LATEST=1 for both;
#                               <lang> is ruby, python, dotnet, jvm, js,
#                               php or go)
#     make <any>:<job> V=1      the same run, verbose, for debugging a failure
#     make quiet T=<target>     the PASS/FAIL treatment for anything else
#
#   Not in verify: the result changes with the day, not with the commit
#     make engine:advisories    the RustSec security advisories against
#                               engine/Cargo.lock; downloads the database.
#                               .github/workflows/advisories.yml runs it on a
#                               change of the lockfile and weekly (not required)
#
#   Build for the SDKs (capi-lib proves only what its symbol check says,
#   napi only that the addon builds, cli-bin only that the CLI prints the
#   workspace version)
#     make engine:capi-lib      the release C ABI library, for the Docker host's
#                               arch, into dist/capi/local/ (gitignored)
#     make engine:napi          the release Node.js addon (the C ABI linked in),
#                               for the Docker host's arch, into
#                               dist/napi/local/xlsx_graft.node (gitignored)
#     make engine:cli-bin       the release CLI, for the Docker host's arch,
#                               into dist/cli/local/xlsx-graft (gitignored)
#
#   Apply fixes
#     make engine:format        rustfmt
#     make engine:licenses-write
#                               write THIRD-PARTY-LICENSES from
#                               engine/Cargo.lock (cargo-about; then: git add
#                               THIRD-PARTY-LICENSES)
#     make sdk:format:jvm       google-java-format, through Spotless, on sdk/jvm
#     make sdk:format:js        Biome's format and safe fixes, on sdk/js
#     make sdk:format:php       php-cs-fixer's fixes, on sdk/php
#     make sdk:format:go        gofmt, on sdk/go
#     make engine:lock          re-resolve engine/Cargo.lock after a manifest
#                               edit — every gate is --locked and refuses until
#                               you do (then: git add engine/Cargo.lock)
#     make engine:update        same, but BUMP within the manifest's ranges
#     make sdk:lock:ruby        re-resolve sdk/ruby/Gemfile.lock after a Gemfile
#                               or gemspec edit (then: git add it)
#     make sdk:lock:python      re-resolve sdk/python/uv.lock after a
#                               pyproject.toml edit (then: git add it)
#     make sdk:lock:dotnet      re-resolve sdk/dotnet's packages.lock.json files
#                               after a csproj edit (then: git add them)
#     make sdk:lock:jvm         re-resolve sdk/jvm's gradle.lockfile and
#                               buildscript-gradle.lockfile after a Gradle build
#                               file edit (then: git add them)
#     make sdk:lock:js          re-resolve sdk/js/pnpm-lock.yaml after a
#                               package.json edit (then: git add it)
#     make sdk:lock:php         re-resolve sdk/php/composer.lock after a
#                               composer.json edit (then: git add it)
#     make sdk:lock:go          re-resolve sdk/go's go.mod files and
#                               tools/go.sum after a go.mod edit (then: git
#                               add sdk/go)
#
#   Open in Excel for Mac (on the host, not in Docker; not a gate, not in
#   verify, never in CI)
#     make excel:check F='<xlsx>...' [CELLS='Sheet1!A1:C3'] [OUT=<file>]
#                               open a copy of each workbook in Excel, record
#                               every dialog (a repair prompt included) and
#                               the displayed text of CELLS, close the copy
#                               without saving, and print JSON
#                               (scripts/excel-check)
#     make fixtures:scrub F='<xlsx>...'
#                               remove the account's name and the local paths
#                               Excel saves in each workbook, in place, before
#                               it goes into fixtures/ (scripts/fixtures-scrub)
#
#   Generate workbooks (in Docker; not a gate, not in verify)
#     make fixtures:generate    write rows-100000.xlsx and rows-1000000.xlsx
#                               into fixtures/generated/ (gitignored) with
#                               rust_xlsxwriter (engine/fixtures-gen)
#
#   Housekeeping (removes things, proves nothing)
#     make clean:all            this tree's engine/target, lcov.info, dist/,
#                               sdk/ruby/coverage/ and .make-logs/ (incl. a
#                               stale gate lock)
#     make docker:clean:all     stopped gate containers, the cargo download
#                               cache volume (and the per-Rust-version volumes
#                               of the layout before the toolchain image), every
#                               pin of the toolchain image, the SDK gate images.
#                               Safe at any time: the next gate pulls the image,
#                               downloads the crates again, and builds nothing
#                               but the engine and the SDK gate images
#
#   Where did it break?
#     cat .make-logs/last-error.log    always the last failed gate (not under
#                                      V=1, not the fix-applying targets)
#
#   What makes these trustworthy — do not defeat it:
#     * NEVER wrap a gate in a pipe, in `; echo $?`, or in `make -n`. A pipe
#       reports the LAST command's status, so it exits 0 over a red gate.
#
# The host has no Rust toolchain: every gate runs inside Docker, in a pinned
# image that already holds everything the gates run (RUST_IMAGE below). The
# only state kept between runs is cargo's download cache, in a named volume.
# The SDK gates run in images built from sdk/<lang>/Dockerfile, never
# published (docs/sdk.md).
#
# GNU Make 4 or newer, because make itself is the one tool that does NOT run in
# a container and so is the one place local and CI can silently disagree. macOS
# still ships 3.81, which strips a backslash-newline inside single quotes before
# the shell sees it; 4.x passes it through. Homebrew's make installs as `gmake`;
# put its gnubin directory first to get it as `make`:
#
#     export PATH="$(brew --prefix make)/libexec/gnubin:$PATH"
#
# A hard error rather than a warning, because a warning on every invocation is
# the shape a reader learns to skip. `help` — the default goal — is exempt, so a
# stock-macOS first contact still gets the target list and this remedy.
ifeq ($(firstword $(subst ., ,$(MAKE_VERSION))),3)
ifneq ($(filter-out help,$(MAKECMDGOALS)),)
$(error GNU Make $(MAKE_VERSION) — CI runs 4.x, and the two differ on recipe quoting: a gate can pass here and fail there. Use Homebrew make: export PATH="$$(brew --prefix make)/libexec/gnubin:$$PATH")
endif
endif

# `make -n` is NOT a dry run here: GNU make still executes recipe lines
# containing $(MAKE), so a "dry" run takes the gate lock and contends with a
# running gate. Refuse it outright — read the recipe, or use `make help`.
#
# `make -i` and `make -t` are refused for the same reason in the other
# direction: -i reaches the nested make and turns a failed step into exit 0,
# and -t skips the recipes, so either prints a green result for a red gate.
# The first word of MAKEFLAGS holds the single-letter flags only.
SHORT_FLAGS := $(firstword -$(MAKEFLAGS))
ifneq (,$(findstring n,$(SHORT_FLAGS)))
$(error `make -n` executes this Makefile's $$(MAKE) delegations for real (it can take the gate lock). Read the recipe instead, or run `make help`)
endif
ifneq (,$(findstring i,$(SHORT_FLAGS))$(findstring t,$(SHORT_FLAGS)))
$(error `make -i` / `make -t` turn a failed gate into exit 0. Run the gate without them)
endif
# `make -e` lets the environment override the knobs (below), which a PASS line
# would then not name.
ifneq (,$(findstring e,$(SHORT_FLAGS)))
$(error `make -e` lets the environment override the knobs. Pass them on the command line: make <target> F=<filter>)
endif

# The toolchain image: docker/toolchain.Dockerfile, published by
# .github/workflows/toolchain-image.yml and pinned here by DIGEST, which a
# rebuild cannot move the way it moves a tag.
#
#   RUST_VERSION, LLVM_COV_VERSION, CARGO_ABOUT_*, CARGO_DENY_*
#                     the pins the workflow builds with. engine:budget fails
#                     when the Dockerfile's ARG defaults say otherwise
#                     (scripts/check-toolchain-pins.sh), and the workflow fails
#                     when the image it pushed reports other tool versions
#                     (scripts/check-toolchain-image.sh)
#   TOOLCHAIN_DIGEST  that build's multi-arch index (amd64 + arm64)
#   TOOLCHAIN_HOLDS   the versions the image at TOOLCHAIN_DIGEST holds, in the
#                     form scripts/check-toolchain-image.sh prints after
#                     `holds:`. engine:verify fails when the image holds other
#                     versions. It is not compared with RUST_VERSION,
#                     LLVM_COV_VERSION, CARGO_ABOUT_VERSION or
#                     CARGO_DENY_VERSION: during a bump, from step 1 until
#                     step 2 below, those name the new image while
#                     TOOLCHAIN_DIGEST still names the previous one. The value
#                     contains commas, so do
#                     not pass it as an argument of $(call), which splits its
#                     arguments at commas
#
# Bumping takes two pull requests, because the image is published from main:
#   1. Change the pins here and the ARG defaults in the Dockerfile, and merge.
#      The workflow pushes the new image and checks both architectures of it.
#   2. Replace the TOOLCHAIN_HOLDS and TOOLCHAIN_DIGEST lines with the two
#      lines under "Paste into the Makefile" in that run's job summary, and
#      merge.
# Until step 2 the gates still run the previous image, and engine:verify still
# passes: it compares the image with TOOLCHAIN_HOLDS, not with the pins above.
# Nothing fails when step 2 is forgotten: the pins above would then name an
# image nobody runs.
# Afterwards `make docker:clean:images` drops the previous image locally.
RUST_VERSION     := 1.99.0
LLVM_COV_VERSION := 0.9.1
# cargo-about writes the license list of the dependencies, cargo-deny checks
# their licenses. The image installs each upstream release binary, checked
# against the per-architecture checksum below.
CARGO_ABOUT_VERSION := 0.9.3
CARGO_ABOUT_SHA256_x86_64 := 3ad1e94ea1e63d69d1961f4fb15ccbbe9575d906111f96d2cb4dce6df48f71b4
CARGO_ABOUT_SHA256_aarch64 := eb4d9ff6ebd2518152a8308729431a04b3836e71d2bd5ead7aea69f91a71162e
CARGO_DENY_VERSION := 0.20.2
CARGO_DENY_SHA256_x86_64 := 9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f
CARGO_DENY_SHA256_aarch64 := 995c82be0defc7a025cae49a2aa2644ce8245c9a3318fc4103907c6a285e8c7d
TOOLCHAIN_HOLDS  := rust 1.99.0, cargo-llvm-cov 0.9.1, cargo-about 0.9.3, cargo-deny 0.20.2
TOOLCHAIN_DIGEST := sha256:0d4bfdc9bff8989fb414d1532c35fcfc650c610e8270dc9004e06c622358495b
RUST_IMAGE       := ghcr.io/kengos/xlsx-graft-toolchain@$(TOOLCHAIN_DIGEST)

# One gate at a time per working tree (scripts/gate-lock.sh explains why).
GATE_LOCK := scripts/gate-lock.sh

# Where `make quiet` parks full gate logs (gitignored). ERROR_LOG is the FIXED
# path a failure always lands at, headed with the target, the exit code and the
# last `== step ==` reached. It is removed when the same command next passes
# (RUN_NAME below), or when a bare `make verify` passes: that runs every gate,
# so it clears any failure, a `make quiet T=<anything>` run's included.
# Otherwise it is the last run that failed, not the tree's state: a narrowed
# run's PASS does not clear a failure of a wider one.
#
# A scope aggregate writes ONE log, named after the command you typed:
# `make engine:verify` writes engine_verify.log and does NOT refresh
# engine_lint.log or engine_coverage.log. Match the log to the command you ran.
LOG_DIR   := .make-logs
ERROR_LOG := $(LOG_DIR)/last-error.log

# Which TREE did that PASS/FAIL describe? A worktree session whose cwd drifts
# back into the primary checkout still finds a Makefile and prints PASS — for
# main. Every gate line names the tree so that drift is visible.
TREE_ID = $(notdir $(CURDIR))@$(shell git -C $(CURDIR) symbolic-ref --quiet --short HEAD 2>/dev/null \
	|| git -C $(CURDIR) rev-parse --short HEAD 2>/dev/null || echo '?')

# Knobs that change WHAT a gate checks (LATEST=, engine:test's P= and F=, an
# SDK's version variable such as RUBY_VER=) are set on the command line only.
# Each is defined with a bare `:=` in the file that reads it, LATEST here as
# every SDK reads it: a makefile assignment overrides the environment and
# loses to the command line, so `export F=x` does nothing, and RUN_NAME
# (MAKEOVERRIDES) names every knob a run had. `make -e` is refused above.
LATEST :=

# Parallelism cap for the heavy gates: `make engine:verify JOBS=4`.
JOBS ?=
CARGO_JOBS := $(if $(JOBS),--jobs $(JOBS))

# ONE NAME PER JOB, and verbosity is a FLAG rather than a second name. Every
# gate is a two-line PUBLIC target that hands off to `gate`; the recipe itself
# lives in a private `_<scope>-<job>` target carrying no `##`, so it never
# reaches `make help`. NAME keeps the private spelling out of the PASS line,
# the log file name and the $(ERROR_LOG) header.
#
# _IN_QUIET is set by `quiet` on the make it starts, so `make quiet T=<a public
# gate>` does not open a second `quiet` that truncates the same log file.
gate = $(if $(V)$(_IN_QUIET),$(GATE_LOCK) )$(MAKE) --no-print-directory $(if $(V)$(_IN_QUIET),$(1),quiet T=$(1) NAME=$(2))

# Run a command in the pinned toolchain image, with CARGO_HOME moved onto a
# volume that keeps cargo's download cache (registry/, git/) between runs. The
# image's own /usr/local/cargo is left unmounted: a named volume over it is
# filled from the image only while empty, so after a digest bump it would keep
# serving the previous image's cargo-llvm-cov, and a bind mount in CI would
# hide cargo itself. Its bin/ stays on PATH, which is where cargo finds the
# rustfmt and clippy proxies and cargo-llvm-cov. Moving the whole CARGO_HOME
# rather than mounting registry/ and git/ alone keeps cargo's package-cache
# lock on the volume as well, so gates of two working trees never write the
# shared cache unlocked.
#
# Nothing in the cache depends on the Rust version, so the name carries none;
# removing it costs the next gate a download, not a build. CARGO_VOLUME is
# overridable: docker takes a bare name as a named volume and an absolute path
# as a host bind mount, which would let CI restore a cache into it.
#
# Mount the REPO ROOT at /repo and let -w select engine/: paths baked in at
# compile time (CARGO_MANIFEST_DIR) then stay identical across runs, and a
# changed mount does not poison the cached test binaries in engine/target.
#
# The label names the working tree, so `make docker:clean:*` can find a
# container this tree left behind — and tell a live gate from an orphan.
#
# --init puts tini at PID 1. Without it PID 1 is `sh`, which ignores the
# SIGINT/SIGTERM docker forwards, so a Ctrl-C'd gate released its lock while
# the container kept writing engine/target under the next gate.
CARGO_VOLUME ?= xlsx-graft-cargo-cache

CARGO_IN_DOCKER = $(GATE_LOCK) docker run --rm --init \
	--label "xlsx-graft.tree=$(CURDIR)" \
	-v "$(CURDIR):/repo" -w /repo/engine \
	-e CARGO_HOME=/cargo-cache -v "$(CARGO_VOLUME):/cargo-cache" \
	$(RUST_IMAGE) sh -euc

.DEFAULT_GOAL := help

# Gates run one after another even under `make -j`: -j reaches the nested make
# that `quiet` starts, the re-entrant gate lock lets it through, and parallel
# members would interleave one log (a wrong `last step`). Cargo's own
# parallelism is JOBS=.
.NOTPARALLEL:

# One file per scope, each holding both halves of its jobs: the public
# `<scope>:<job>` wrappers and the private `_<scope>-<job>` recipes; the SDKs'
# per-language jobs are in a file per language, after mk/sdk.mk, whose
# machinery they use. What stays in THIS file is the shared machinery above and
# `help`/`quiet`/`clean`.
include mk/engine.mk
include mk/engine/licenses.mk
include mk/sdk.mk
include mk/sdk/ruby.mk
include mk/sdk/python.mk
include mk/sdk/dotnet.mk
include mk/sdk/jvm.mk
include mk/sdk/js.mk
include mk/sdk/php.mk
include mk/sdk/go.mk
include mk/docker.mk
include mk/excel.mk

.PHONY: _verify ci\:matrix help quiet verify clean clean\:all clean\:target

help: ## Show this help
	@grep -hE '^[a-zA-Z0-9_\\:-]+:.*## ' $(MAKEFILE_LIST) \
		| sed -E 's/^([a-zA-Z0-9_\\:-]+):[^#]*## /\1\t/; s/\\//g' \
		| awk -F'\t' '{printf "  \033[36m%-24s\033[0m %s\n", $$1, $$2}'

# The run's name is the command you typed: the public target (NAME, which
# defaults to T so `make quiet T=<x>` is unchanged) plus every command-line
# variable in the order given (MAKEOVERRIDES, which the nested makes inherit),
# minus the variables that change HOW a gate runs, not WHAT it checks. make
# does not decide whether a variable narrowed the gate: a PASS line that
# carries one is a run with that variable, and the bar is the bare name. A
# value must not hold a `'`.
HARNESS_VARS := T NAME V _IN_QUIET JOBS CARGO_VOLUME
RUN_ARGS = $(strip $(filter-out $(addsuffix =%,$(HARNESS_VARS)),$(MAKEOVERRIDES)))
RUN_NAME = $(strip $(or $(NAME),$(T)) $(RUN_ARGS))

quiet: ## Run ANY target this way: make quiet T=engine:test (same PASS/FAIL line; non-zero on failure)
	@if [ -z "$(T)" ]; then \
		echo "usage: make quiet T=<target>    e.g. make quiet T=engine:verify" >&2; \
		exit 2; \
	fi
	@mkdir -p $(LOG_DIR)
	@# Each run writes its own <log>.<pid> and moves it into place at the end:
	@# the redirect opens the file BEFORE the gate lock is asked, so a second
	@# gate refused by the lock would otherwise truncate the running one's log.
	@name='$(RUN_NAME)'; log="$(LOG_DIR)/$$(printf '%s' "$$name" | tr ' :/\\' '____').log"; tmp="$$log.$$$$"; \
	if $(GATE_LOCK) $(MAKE) --no-print-directory $(T) _IN_QUIET=1 > "$$tmp" 2>&1; then code=0; else code=$$?; fi; \
	mv -f "$$tmp" "$$log"; \
	if [ $$code -eq 0 ]; then \
		if [ -f "$(ERROR_LOG)" ] && { [ "$$name" = verify ] \
			|| head -1 "$(ERROR_LOG)" | grep -qxF "# FAILED: $$name"; }; then \
			rm -f "$(ERROR_LOG)"; \
		fi; \
		printf '\033[32mPASS\033[0m %s  [%s] — full log (%s lines): %s\n' \
			"$$name" '$(TREE_ID)' "$$(wc -l < "$$log" | tr -d ' ')" "$$log"; \
	else \
		step=$$(grep -E '^== ' "$$log" | tail -1); \
		{ echo "# FAILED: $$name"; \
		  echo "# tree: $(TREE_ID) ($(CURDIR))"; \
		  echo "# exit $$code at $$(date '+%Y-%m-%d %H:%M:%S')"; \
		  echo "# last step: $${step:-(none reported)}"; \
		  echo "# full log: $$log"; \
		  echo; cat "$$log"; } > "$(ERROR_LOG)"; \
		printf '\033[31mFAIL\033[0m %s  [%s] (exit %s)\n  last step : %s\n  error log : %s  <- always this path\n  full log  : %s\n' \
			"$$name" '$(TREE_ID)' "$$code" "$${step:-(none reported)}" "$(ERROR_LOG)" "$$log"; \
		printf -- '--- last 40 lines ---\n'; tail -40 "$$log"; \
		exit $$code; \
	fi

## ---- the whole bar ----------------------------------------------------
#
# One target over every scope, so CI and a contributor run the same thing.
# Each scope's `<scope>:verify` joins it as a member of `_verify`, every SDK on
# every line it supports. LATEST=1 narrows each SDK to its latest line, for a
# faster run while iterating; CI never sets it.
#
# CI runs the same members as parallel jobs rather than one `make verify`:
# `ci:matrix` prints them, one for the engine and one per SDK line, and
# .github/workflows/ci.yml's required `make verify` job passes only when all of
# them do. A scope that joins `_verify` joins CI_MATRIX too.

verify: ## The whole bar: every scope's verify, every SDK on every supported line (LATEST=1: the latest line only)
	@$(call gate,_verify,verify)

_verify: _engine-verify _sdk-verify

CI_MATRIX = {"include":[{"target":"engine:verify","args":""}$(SDK_CI_JOBS)]}

# Not a gate: it checks nothing and prints one line of JSON, which CI reads.
ci\:matrix: ## Print verify's members as CI's job matrix (JSON, one job for the engine and one per SDK line)
	@echo '$(CI_MATRIX)'

## ---- housekeeping ------------------------------------------------------
#
# Files inside THIS working tree only; the Docker resources are docker:clean:*.

clean: ## Remove engine/lcov.info
	rm -f engine/lcov.info

clean\:target: ## Remove this tree's engine/target (refuses while a gate runs)
	@scripts/clean-tree.sh target

clean\:all: ## Remove this tree's engine/target, engine/lcov.info, dist/, sdk/ruby/coverage/ and .make-logs/ (incl. a stale gate lock)
	@scripts/clean-tree.sh all
