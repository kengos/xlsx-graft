# sdk/js — the JS SDK's gates and lockfile (mk/sdk.mk names the values a
# language's file sets and what `sdk_lang` makes of them)

.PHONY: _sdk-image-js _sdk-lint-js _sdk-lint-js-line _sdk-test-js _sdk-test-js-line \
        _sdk-verify-js _sdk-verify-js-line sdk\:lint\:js sdk\:lock\:js sdk\:test\:js \
        sdk\:verify\:js

# The floor first and the latest stable line last: LATEST=1 takes the last and
# sdk:lock:js the first. Each line is pinned to a Node.js release. The base
# image's Debian layers still move when Docker Hub rebuilds the tag.
JS_LINES := 24 26
JS_PATCH_24 := 24.22.1
JS_PATCH_26 := 26.12.1
# Command line only (Makefile § knobs).
JS_VER :=
JS_IMAGE_INPUTS := $(addprefix sdk/js/,Dockerfile package.json pnpm-lock.yaml pnpm-workspace.yaml)
# Node.js has no FFI on these lines, so the SDK loads the addon that links the
# C ABI in (engine:napi), not the C ABI's library.
JS_ENGINE := XLSX_GRAFT_ADDON=/repo/$(NAPI_LOCAL)/xlsx_graft.node
$(eval $(call sdk_lang,js,JS))

# Writes sdk/js/pnpm-lock.yaml and does NOT go through `quiet`, like
# engine:lock. It runs on the Dockerfile's `tools` stage, Node.js and pnpm
# without the tools: the gate image installs from the lockfile, so it cannot
# be the one that makes it. pnpm resolves in a copy under /tmp, so no
# node_modules or pnpm state lands in the checkout, and only the lockfile is
# copied back. The lockfile holds every platform's optional binaries, so one
# serves x86-64 and arm64. It runs on the floor only as sdk:lock:ruby does;
# whether every line installs the lock is what that line's gate checks.
sdk\:lock\:js: override JS_VER = $(firstword $(JS_LINES))
sdk\:lock\:js: ## Re-resolve sdk/js/pnpm-lock.yaml after a package.json edit (then: git add sdk/js/pnpm-lock.yaml)
	@echo "== sdk:lock:js (Node.js $(JS_PATCH)) =="
	docker build -q --target tools --build-arg JS_PATCH=$(JS_PATCH) -t $(JS_IMAGE)-tools sdk/js >/dev/null
	$(GATE_LOCK) docker run --rm --init --label "xlsx-graft.tree=$(CURDIR)" \
		-v "$(CURDIR):/repo" -w /repo/sdk/js $(JS_IMAGE)-tools sh -euc \
		'mkdir /tmp/lock; \
		cp package.json pnpm-workspace.yaml /tmp/lock/; \
		if [ -f pnpm-lock.yaml ]; then cp pnpm-lock.yaml /tmp/lock/; fi; \
		cd /tmp/lock; \
		pnpm install --lockfile-only; \
		cp pnpm-lock.yaml /repo/sdk/js/pnpm-lock.yaml'

_sdk-image-js:
	@echo "== sdk js image (Node.js $(JS_PATCH)) =="
	@docker build -q --build-arg JS_PATCH=$(JS_PATCH) -t $(JS_IMAGE) sdk/js >/dev/null

# Every step runs on a copy of the checkout under /tmp with the image's
# node_modules linked in, so nothing lands in the mounted checkout: no
# node_modules link, dist/ or coverage/ owned by root. The copy sits at
# /tmp/repo/sdk/js beside a link to the engine, so the tests read
# engine/Cargo.toml at the path they would in the checkout. Whatever a host
# install left in the copy (node_modules, dist/, coverage/, native/) is
# removed first.
JS_CHECKOUT := mkdir -p /tmp/repo/sdk; \
	cp -r /repo/sdk/js /tmp/repo/sdk/js; \
	ln -s /repo/engine /tmp/repo/engine; \
	cd /tmp/repo/sdk/js; \
	rm -rf node_modules dist coverage native; \
	ln -s /pkg/node_modules node_modules
# One command per step under `sh -e`, so the first red one stops the rest.
JS_LINT := pnpm run lint; pnpm run typecheck
JS_TEST := pnpm run test

# Lint, the tests at 100% line coverage, then the package as a user gets it:
# built in a copy of the checkout installed from the lockfile out of the
# image's store alone (--offline), with the workspace's license files beside
# it, and packed; the tarball installed alone into a project of its own
# (script/check-installed-package, copied out of the checkout too) and loaded
# from there on the line's Node.js, with `import` and with `require()`.
sdk\:verify\:js: ## Verify sdk/js on every supported line — Biome + tsc + Vitest at 100% line coverage + npm pack/install/load (LATEST=1, JS_VER=<line>)
	@$(call gate,_sdk-verify-js,sdk:verify:js)

_sdk-verify-js: _engine-napi
	@$(call js_each,_sdk-verify-js-line)

_sdk-verify-js-line: _sdk-image-js
	@echo "== sdk js lint + test (Node.js $(JS_PATCH)) =="
	$(JS_IN_DOCKER) '$(JS_CHECKOUT); $(JS_LINT); $(JS_TEST)'
	@echo "== sdk js package (pack, install, load) (Node.js $(JS_PATCH)) =="
	$(JS_IN_DOCKER) 'cp -r /repo/sdk/js /tmp/build; \
		cd /tmp/build; \
		rm -rf node_modules dist coverage native; \
		cp /repo/LICENSE-APACHE /repo/LICENSE-BSD /repo/LICENSE-MIT .; \
		pnpm install --offline --frozen-lockfile; \
		pnpm run build; \
		pnpm pack --pack-destination /tmp/pack; \
		cp -r /repo/sdk/js/script/check-installed-package /tmp/user; \
		cd /tmp/user; \
		npm install --offline --ignore-scripts /tmp/pack/xlsx-graft-*.tgz; \
		node check.mjs /tmp/user /repo/engine/Cargo.toml $(JS_PATCH)'

sdk\:test\:js: ## sdk/js Vitest at 100% line coverage, on every supported line (LATEST=1, JS_VER=<line>)
	@$(call gate,_sdk-test-js,sdk:test:js)

_sdk-test-js: _engine-napi
	@$(call js_each,_sdk-test-js-line)

_sdk-test-js-line: _sdk-image-js
	@echo "== sdk js test (Node.js $(JS_PATCH)) =="
	$(JS_IN_DOCKER) '$(JS_CHECKOUT); $(JS_TEST)'

# No engine addon needed, so no napi: lint runs without a Rust build.
sdk\:lint\:js: ## sdk/js Biome (lint + format) and tsc --noEmit, on every supported line (LATEST=1, JS_VER=<line>)
	@$(call gate,_sdk-lint-js,sdk:lint:js)

_sdk-lint-js:
	@$(call js_each,_sdk-lint-js-line)

_sdk-lint-js-line: _sdk-image-js
	@echo "== sdk js lint (Node.js $(JS_PATCH)) =="
	$(JS_IN_DOCKER) '$(JS_CHECKOUT); $(JS_LINT)'
