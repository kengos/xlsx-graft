# sdk/go — the Go SDK's gates and lockfile (mk/sdk.mk names the values a
# language's file sets and what `sdk_lang` makes of them)

.PHONY: _sdk-image-go _sdk-lint-go _sdk-lint-go-line _sdk-test-go _sdk-test-go-line \
        _sdk-verify-go _sdk-verify-go-line sdk\:lint\:go sdk\:lock\:go sdk\:test\:go \
        sdk\:verify\:go

# The floor first and the latest stable line last: LATEST=1 takes the last and
# sdk:lock:go the first. Go supports its two latest lines, so the older one is
# always within six months of its end; rather than leave Go with its newest
# line alone, both are gated (docs/sdk.md § Supported language versions).
# Each line is pinned to a Go release. The base image's Debian layers still
# move when Docker Hub rebuilds the tag.
GO_LINES := 1.26 1.27
GO_PATCH_1.26 := 1.26.10
GO_PATCH_1.27 := 1.27.3
# Command line only (Makefile § knobs).
GO_VER :=
GO_IMAGE_INPUTS := $(addprefix sdk/go/,Dockerfile tools/go.mod tools/go.sum)
# The SDK runs the engine's CLI (engine:cli-bin) as a subprocess rather than
# loading the C ABI's library through cgo, which would leave every user's
# build needing a C toolchain.
GO_ENGINE := XLSX_GRAFT_CLI=/repo/$(CLI_LOCAL)/xlsx-graft
$(eval $(call sdk_lang,go,GO))

# Writes sdk/go/go.mod, sdk/go/tools/go.mod and sdk/go/tools/go.sum, and does
# NOT go through `quiet`, like engine:lock. It runs on the Dockerfile's
# `tools` stage, Go without golangci-lint: the gate image builds the linter
# from tools/go.sum, so it cannot be the one that makes it. `go mod tidy`
# runs in a copy under /tmp for both modules, the SDK's (which should require
# nothing) and tools/, and only their go.mod and go.sum files are copied
# back. go.sum holds every platform's hashes, so one serves x86-64 and arm64.
# It runs on the floor only as sdk:lock:ruby does; whether every line builds
# the locked linter is what that line's gate checks.
sdk\:lock\:go: override GO_VER = $(firstword $(GO_LINES))
sdk\:lock\:go: ## Re-resolve sdk/go's go.mod files and tools/go.sum after a go.mod edit (then: git add sdk/go)
	@echo "== sdk:lock:go (Go $(GO_PATCH)) =="
	docker build -q --target tools --build-arg GO_PATCH=$(GO_PATCH) -t $(GO_IMAGE)-tools sdk/go >/dev/null
	$(GATE_LOCK) docker run --rm --init --label "xlsx-graft.tree=$(CURDIR)" \
		-v "$(CURDIR):/repo" -w /repo/sdk/go $(GO_IMAGE)-tools sh -euc \
		'cp -r /repo/sdk/go /tmp/lock; \
		cd /tmp/lock; \
		go mod tidy; \
		cd tools; \
		go mod tidy; \
		cd /tmp/lock; \
		for f in go.mod go.sum tools/go.mod tools/go.sum; do \
			if [ -f "$$f" ]; then cp "$$f" "/repo/sdk/go/$$f"; fi; \
		done'

_sdk-image-go:
	@echo "== sdk go image (Go $(GO_PATCH)) =="
	@docker build -q --build-arg GO_PATCH=$(GO_PATCH) -t $(GO_IMAGE) sdk/go >/dev/null

# Every step runs on a copy of the checkout under /tmp, so nothing lands in the
# mounted checkout: the cover profile is written under /tmp too. The copy sits
# at /tmp/repo/sdk/go beside a link to the engine, so the tests read
# engine/Cargo.toml at the path they would in the checkout.
GO_CHECKOUT := mkdir -p /tmp/repo/sdk; \
	cp -r /repo/sdk/go /tmp/repo/sdk/go; \
	ln -s /repo/engine /tmp/repo/engine; \
	cd /tmp/repo/sdk/go
# One command per step under `sh -e`, so the first red one stops the rest.
# `gofmt -l` lists what it would rewrite and exits 0 either way, so the list
# has to be empty. `./...` stops at a directory with a go.mod of its own
# (tools/, script/check-installed-module/); gofmt reads those too. The second
# vet is Windows': what builds there alone (signal_other.go) is compiled by no
# other step.
GO_LINT := unformatted=$$(gofmt -l .); \
	if [ -n "$$unformatted" ]; then echo "gofmt would rewrite:" >&2; echo "$$unformatted" >&2; exit 1; fi; \
	go vet ./...; \
	GOOS=windows go vet ./...; \
	golangci-lint run ./...
# `go test` has no fail-under flag, so the 100% bar is read from the last
# line of `go tool cover -func`, the total.
GO_TEST := go test -race -coverprofile=/tmp/cover.out ./...; \
	total=$$(go tool cover -func=/tmp/cover.out | tail -n 1); \
	echo "$$total"; \
	case "$$total" in total:*100.0%) ;; *) echo "coverage is under 100%" >&2; exit 1;; esac

# Lint, the tests at 100% statement coverage, then the module as a user gets it.
# Go has no package to build: a release is the module's directory, zipped by
# the proxy from the tag. So what that zip holds is copied out of the checkout
# to /tmp/build, every directory with a go.mod of its own (tools/,
# script/check-installed-module/) left out as the zip leaves it out, and a
# module of its own (script/check-installed-module, copied out of the
# checkout too) requires it through a `replace` with no proxy to reach
# (GOPROXY=off), is built, and runs on the line's Go.
sdk\:verify\:go: ## Verify sdk/go on every supported line — gofmt + go vet + golangci-lint + go test -race at 100% statement coverage + module copy/build/run (LATEST=1, GO_VER=<line>)
	@$(call gate,_sdk-verify-go,sdk:verify:go)

_sdk-verify-go: _engine-cli-bin
	@$(call go_each,_sdk-verify-go-line)

_sdk-verify-go-line: _sdk-image-go
	@echo "== sdk go lint + test (Go $(GO_PATCH)) =="
	$(GO_IN_DOCKER) '$(GO_CHECKOUT); $(GO_LINT); $(GO_TEST)'
	@echo "== sdk go module (copy, build, run) (Go $(GO_PATCH)) =="
	$(GO_IN_DOCKER) 'cp -r /repo/sdk/go /tmp/build; \
		cd /tmp/build; \
		find . -mindepth 2 -name go.mod -exec dirname {} + > /tmp/nested; \
		xargs rm -rf < /tmp/nested; \
		cp -r /repo/sdk/go/script/check-installed-module /tmp/user; \
		cd /tmp/user; \
		go build -o /tmp/check .; \
		/tmp/check /tmp/build /repo/engine/Cargo.toml $(GO_PATCH)'

sdk\:test\:go: ## sdk/go go test -race at 100% statement coverage, on every supported line (LATEST=1, GO_VER=<line>)
	@$(call gate,_sdk-test-go,sdk:test:go)

_sdk-test-go: _engine-cli-bin
	@$(call go_each,_sdk-test-go-line)

_sdk-test-go-line: _sdk-image-go
	@echo "== sdk go test (Go $(GO_PATCH)) =="
	$(GO_IN_DOCKER) '$(GO_CHECKOUT); $(GO_TEST)'

# No engine CLI needed, so no cli-bin: lint runs without a Rust build.
sdk\:lint\:go: ## sdk/go gofmt, go vet and golangci-lint, on every supported line (LATEST=1, GO_VER=<line>)
	@$(call gate,_sdk-lint-go,sdk:lint:go)

_sdk-lint-go:
	@$(call go_each,_sdk-lint-go-line)

_sdk-lint-go-line: _sdk-image-go
	@echo "== sdk go lint (Go $(GO_PATCH)) =="
	$(GO_IN_DOCKER) '$(GO_CHECKOUT); $(GO_LINT)'
