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
#                               engine:verify only)
#     make engine:verify        engine: budget + toolchain pins + fmt + clippy +
#                               tests at 100% line coverage + doctests + C ABI
#                               symbols, in the pinned toolchain image
#
#   Faster slices while iterating
#     make engine:budget        make engine:fmt         make engine:clippy
#     make engine:lint          make engine:test [P=<crate>] [F=<filter>]
#     make engine:coverage      make engine:capi-abi
#     make <any>:<job> V=1      the same run, verbose, for debugging a failure
#     make quiet T=<target>     the PASS/FAIL treatment for anything else
#
#   Apply fixes
#     make engine:format        rustfmt
#     make engine:lock          re-resolve engine/Cargo.lock after a manifest
#                               edit — every gate is --locked and refuses until
#                               you do (then: git add engine/Cargo.lock)
#     make engine:update        same, but BUMP within the manifest's ranges
#
#   Housekeeping (removes things, proves nothing)
#     make clean:all            this tree's engine/target, lcov.info and
#                               .make-logs/ (incl. a stale gate lock)
#     make docker:clean:all     stopped gate containers, the cargo download
#                               cache volume (and the per-Rust-version volumes
#                               of the layout before the toolchain image), every
#                               pin of the toolchain image. Safe at any time:
#                               the next gate pulls the image and downloads the
#                               crates again, and builds nothing but the engine
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

# The toolchain image: docker/toolchain.Dockerfile, published by
# .github/workflows/toolchain-image.yml and pinned here by DIGEST, which a
# rebuild cannot move the way it moves a tag.
#
#   RUST_VERSION, LLVM_COV_VERSION  the pins the workflow builds with.
#                     engine:budget fails when the Dockerfile's ARG defaults
#                     say otherwise (scripts/check-toolchain-pins.sh)
#   TOOLCHAIN_TAG     the tag TOOLCHAIN_DIGEST was published under,
#                     <rust>-llvmcov<cargo-llvm-cov>. engine:verify fails when
#                     the image does not hold those versions
#   TOOLCHAIN_DIGEST  that build's multi-arch index (amd64 + arm64)
#
# Bumping takes two pull requests, because the image is published from main:
#   1. Change RUST_VERSION / LLVM_COV_VERSION here and the ARG defaults in the
#      Dockerfile, and merge. The workflow pushes the new image.
#   2. Copy the digest from that run's job summary into TOOLCHAIN_DIGEST, set
#      TOOLCHAIN_TAG to the new pins, and merge.
# Until step 2 the gates still run the previous image. Nothing fails when step
# 2 is forgotten: the pins above would then name an image nobody runs.
# Afterwards `make docker:clean:images` drops the previous image locally.
RUST_VERSION     := 1.99.0
LLVM_COV_VERSION := 0.9.1
TOOLCHAIN_TAG    := 1.99.0-llvmcov0.9.1
TOOLCHAIN_DIGEST := sha256:05819e6c259e11dd9d8b0cdcd178eb959dbaccb90923d56cf98a6b66885e4fe0
RUST_IMAGE       := ghcr.io/kengos/xlsx-graft-toolchain@$(TOOLCHAIN_DIGEST)

# One gate at a time per working tree (scripts/gate-lock.sh explains why).
GATE_LOCK := scripts/gate-lock.sh

# Where `make quiet` parks full gate logs (gitignored). ERROR_LOG is the FIXED
# path a failure always lands at, headed with the target, the exit code and the
# last `== step ==` reached. It is removed when the same target next passes.
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
# `<scope>:<job>` wrappers and the private `_<scope>-<job>` recipes. What stays
# in THIS file is the shared machinery above and `help`/`quiet`/`clean`.
include mk/engine.mk
include mk/docker.mk

.PHONY: _verify help quiet verify clean clean\:all clean\:target

help: ## Show this help
	@grep -hE '^[a-zA-Z0-9_\\:-]+:.*## ' $(MAKEFILE_LIST) \
		| sed -E 's/^([a-zA-Z0-9_\\:-]+):[^#]*## /\1\t/; s/\\//g' \
		| awk -F'\t' '{printf "  \033[36m%-24s\033[0m %s\n", $$1, $$2}'

# NAME labels the run; it defaults to T so `make quiet T=<x>` is unchanged.
RUN_NAME = $(if $(NAME),$(NAME),$(T))

quiet: ## Run ANY target this way: make quiet T=engine:test (same PASS/FAIL line; non-zero on failure)
	@if [ -z "$(T)" ]; then \
		echo "usage: make quiet T=<target>    e.g. make quiet T=engine:verify" >&2; \
		exit 2; \
	fi
	@mkdir -p $(LOG_DIR)
	@# Each run writes its own <log>.<pid> and moves it into place at the end:
	@# the redirect opens the file BEFORE the gate lock is asked, so a second
	@# gate refused by the lock would otherwise truncate the running one's log.
	@log="$(LOG_DIR)/$$(echo '$(RUN_NAME)' | tr ' :/' '___').log"; tmp="$$log.$$$$"; \
	if $(GATE_LOCK) $(MAKE) --no-print-directory $(T) _IN_QUIET=1 > "$$tmp" 2>&1; then code=0; else code=$$?; fi; \
	mv -f "$$tmp" "$$log"; \
	if [ $$code -eq 0 ]; then \
		if [ -f "$(ERROR_LOG)" ] && head -1 "$(ERROR_LOG)" | grep -qx "# FAILED: $(RUN_NAME)"; then \
			rm -f "$(ERROR_LOG)"; \
		fi; \
		printf '\033[32mPASS\033[0m %s  [%s] — full log (%s lines): %s\n' \
			'$(RUN_NAME)' '$(TREE_ID)' "$$(wc -l < "$$log" | tr -d ' ')" "$$log"; \
	else \
		step=$$(grep -E '^== ' "$$log" | tail -1); \
		{ echo "# FAILED: $(RUN_NAME)"; \
		  echo "# tree: $(TREE_ID) ($(CURDIR))"; \
		  echo "# exit $$code at $$(date '+%Y-%m-%d %H:%M:%S')"; \
		  echo "# last step: $${step:-(none reported)}"; \
		  echo "# full log: $$log"; \
		  echo; cat "$$log"; } > "$(ERROR_LOG)"; \
		printf '\033[31mFAIL\033[0m %s  [%s] (exit %s)\n  last step : %s\n  error log : %s  <- always this path\n  full log  : %s\n' \
			'$(RUN_NAME)' '$(TREE_ID)' "$$code" "$${step:-(none reported)}" "$(ERROR_LOG)" "$$log"; \
		printf -- '--- last 40 lines ---\n'; tail -40 "$$log"; \
		exit $$code; \
	fi

## ---- the whole bar ----------------------------------------------------
#
# One target over every scope, so CI and a contributor run the same thing.
# Each scope's `<scope>:verify` joins it as a member of `_verify`; engine/ is
# the only scope today.

verify: ## The whole bar: every scope's verify (today: engine:verify)
	@$(call gate,_verify,verify)

_verify: _engine-verify

## ---- housekeeping ------------------------------------------------------
#
# Files inside THIS working tree only; the Docker resources are docker:clean:*.

clean: ## Remove engine/lcov.info
	rm -f engine/lcov.info

clean\:target: ## Remove this tree's engine/target (refuses while a gate runs)
	@scripts/clean-tree.sh target

clean\:all: ## Remove this tree's engine/target, engine/lcov.info and .make-logs/ (incl. a stale gate lock)
	@scripts/clean-tree.sh all
