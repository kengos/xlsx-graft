# sdk/ — the language SDKs: gates and lockfiles
#
# The public targets are named `sdk:<job>` (every language, defined here) and
# `sdk:<job>:<lang>` (in mk/sdk/<lang>.mk); the private ones, carrying the
# recipes, `_sdk-<job>[-<lang>[-line]]`, the `-line` one running a single
# release line. docs/sdk.md is the shape every language follows.
#
# A gate runs every release line a language supports, one after another;
# LATEST=1 runs the latest stable line only (docs/sdk.md § Supported language
# versions).
#
# A language's file, mk/sdk/<lang>.mk, holds its values under names of the form
# <LANG>_<what> and hands them to `sdk_lang` (below), which defines the
# machinery every language shares from them. What the file provides, with
# Ruby's values:
#
#   # The supported release lines, the floor first and the latest stable line
#   # last, each pinned to a patch release, so the same commit gates on the
#   # same interpreter.
#   RUBY_LINES := 3.4 4.0
#   RUBY_PATCH_3.4 := 3.4.11
#   RUBY_PATCH_4.0 := 4.0.7
#   # The line one run gates. Command line only (Makefile § knobs), so a bare
#   # `:=`.
#   RUBY_VER :=
#   # The files sdk/<lang>/Dockerfile builds the gate image from, all of them:
#   # their hash is in the image's tag.
#   RUBY_IMAGE_INPUTS := $(addprefix sdk/ruby/,Dockerfile Gemfile Gemfile.lock …)
#   $(eval $(call sdk_lang,ruby,RUBY))
#
# and then its own recipes: the image build (`_sdk-image-<lang>`, tagging it
# $(<LANG>_IMAGE)), the four public targets of docs/sdk.md § The gate, per
# language, and their private `_sdk-<job>-<lang>[-line]` recipes.

.PHONY: _sdk-lint _sdk-test _sdk-verify sdk\:lint sdk\:test sdk\:verify

## ---- every language ----------------------------------------------------

# Every SDK, by the <lang> of its targets. A language joins here, which puts it
# in sdk:verify / test / lint and, through its CI_JOBS_<lang>, in CI.
SDK_LANGS := ruby python dotnet jvm

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

## ---- per language -----------------------------------------------------

# $(eval $(call sdk_lang,<lang>,<LANG>)): what every language shares, read from
# the <LANG>_* values its file set before the call. It defines:
#
#   <LANG>_GATED       the lines a gate runs: <LANG>_VER's alone, the latest
#                      under LATEST=1, otherwise every one
#   <LANG>_PATCH       the patch release of <LANG>_VER's line, for the recipes
#                      of one line, which need it set
#   CI_JOBS_<lang>     one `sdk:verify:<lang> <LANG>_VER=<line>` per line. Each
#                      entry opens with the comma that joins it to the one
#                      before (CI_MATRIX, Makefile)
#   <lang>_each        $(call <lang>_each,<target>) runs <target> once per
#                      gated line, each in a make of its own: <LANG>_VER fixes
#                      the image every recipe names, and one make runs a target
#                      once. The first red line stops the rest
#   <LANG>_IMAGE       the gate image's tag (below)
#   <LANG>_IN_DOCKER   the `docker run` a recipe hands its shell script to, in
#                      sdk/<lang> with the engine library mounted (below)
#
# and refuses a <LANG>_VER outside <LANG>_LINES when make reads the file, before
# any gate builds the engine. It also refuses a language file that set its
# values after the call, or not at all: no lines would gate nothing and still
# pass, no image inputs would leave `cat` reading the terminal, and no bare
# `<LANG>_VER :=` would let the environment narrow a gate its PASS line calls
# whole.
#
# The gate image is built from sdk/<lang>/Dockerfile on every gate and never
# published: it holds the locked dependencies alone, and docker's layer cache
# keeps a rebuild to a no-op until the lockfile changes. The tag carries a hash
# of the files the image is built from, because gates of two working trees may
# run at once: under one shared tag, a tree whose lockfile differs would run on
# the other tree's dependencies.
#
# The engine library comes from the mount, not the image (docs/sdk.md § The
# gate, per language). Inside Docker it is always the Linux .so. The same
# container label and --init as CARGO_IN_DOCKER; --pull never because the image
# exists only locally, and a missing one is a local error rather than a failed
# pull.
define sdk_lang
$$(if $$($(2)_LINES),,$$(error mk/sdk/$(1).mk sets no $(2)_LINES before calling sdk_lang))
$$(foreach v,$$($(2)_LINES),$$(if $$($(2)_PATCH_$$(v)),,$$(error mk/sdk/$(1).mk sets no $(2)_PATCH_$$(v) before calling sdk_lang)))
$$(if $$($(2)_IMAGE_INPUTS),,$$(error mk/sdk/$(1).mk sets no $(2)_IMAGE_INPUTS before calling sdk_lang))
$$(if $$(filter file command,$$(origin $(2)_VER)),,$$(error mk/sdk/$(1).mk sets no bare $(2)_VER := before calling sdk_lang))

$(2)_GATED = $$(or $$($(2)_VER),$$(if $$(LATEST),$$(lastword $$($(2)_LINES)),$$($(2)_LINES)))
$(2)_PATCH = $$(or $$($(2)_PATCH_$$($(2)_VER)),$$(error $(2)_VER is unset; run a public sdk:*:$(1) target))

CI_JOBS_$(1) = $$(foreach v,$$($(2)_LINES),,{"target":"sdk:verify:$(1)","args":"$(2)_VER=$$(v)"})

ifneq ($$(filter-out $$($(2)_LINES),$$($(2)_VER)),)
$$(error $(2)_VER=$$($(2)_VER) is not a gated line; use one of $$($(2)_LINES))
endif

$(1)_each = for v in $$($(2)_GATED); do $$(MAKE) --no-print-directory $$(1) $(2)_VER=$$$$v || exit; done

$(2)_IMAGE_HASH := $$(shell cat $$($(2)_IMAGE_INPUTS) | git hash-object --stdin | cut -c1-12)
$(2)_IMAGE = xlsx-graft-sdk-$(1):$$($(2)_PATCH)-$$($(2)_IMAGE_HASH)

$(2)_IN_DOCKER = $$(GATE_LOCK) docker run --rm --init --pull never \
	--label "xlsx-graft.tree=$$(CURDIR)" \
	-v "$$(CURDIR):/repo" -w /repo/sdk/$(1) \
	-e XLSX_GRAFT_LIBRARY=/repo/$$(CAPI_LOCAL)/libxlsx_graft_capi.so \
	$$($(2)_IMAGE) sh -euc
endef
