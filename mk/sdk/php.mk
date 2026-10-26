# sdk/php — the PHP SDK's gates and lockfile (mk/sdk.mk names the values a
# language's file sets and what `sdk_lang` makes of them)

.PHONY: _sdk-image-php _sdk-lint-php _sdk-lint-php-line _sdk-test-php _sdk-test-php-line \
        _sdk-verify-php _sdk-verify-php-line sdk\:lint\:php sdk\:lock\:php sdk\:test\:php \
        sdk\:verify\:php

# The floor first and the latest stable line last: LATEST=1 takes the last and
# sdk:lock:php the first. Each line is pinned to a PHP release. The base
# image's Debian layers still move when Docker Hub rebuilds the tag.
PHP_LINES := 8.3 8.4 8.5
PHP_PATCH_8.3 := 8.3.36
PHP_PATCH_8.4 := 8.4.27
PHP_PATCH_8.5 := 8.5.12
# Command line only (Makefile § knobs).
PHP_VER :=
PHP_IMAGE_INPUTS := $(addprefix sdk/php/,Dockerfile composer.json composer.lock)
# PHP has no FFI it can count on (ext-ffi is rarely enabled), so the SDK runs
# the engine's CLI (engine:cli-bin) as a subprocess rather than loading the C
# ABI's library.
PHP_ENGINE := XLSX_GRAFT_CLI=/repo/$(CLI_LOCAL)/xlsx-graft
$(eval $(call sdk_lang,php,PHP))

# Writes sdk/php/composer.lock and does NOT go through `quiet`, like
# engine:lock. It runs on the Dockerfile's `tools` stage, PHP and Composer
# without the tools: the gate image installs from the lockfile, so it cannot
# be the one that makes it. Composer resolves in a copy under /tmp, so no
# vendor/ lands in the checkout, and only the lockfile is copied back.
# --minimal-changes keeps every locked version the edit leaves satisfiable.
# composer.json's config.platform.php resolves for the floor whatever PHP
# runs it, and the lock holds no platform-specific package, so one serves
# x86-64 and arm64. It runs on the floor only as sdk:lock:ruby does. The same
# platform setting holds every line's install to the floor's PHP too, so what
# a later line's gate checks is that the locked tools run on that PHP.
sdk\:lock\:php: override PHP_VER = $(firstword $(PHP_LINES))
sdk\:lock\:php: ## Re-resolve sdk/php/composer.lock after a composer.json edit (then: git add sdk/php/composer.lock)
	@echo "== sdk:lock:php (PHP $(PHP_PATCH)) =="
	docker build -q --target tools --build-arg PHP_PATCH=$(PHP_PATCH) -t $(PHP_IMAGE)-tools sdk/php >/dev/null
	$(GATE_LOCK) docker run --rm --init --label "xlsx-graft.tree=$(CURDIR)" \
		-v "$(CURDIR):/repo" -w /repo/sdk/php $(PHP_IMAGE)-tools sh -euc \
		'mkdir /tmp/lock; \
		cp composer.json /tmp/lock/; \
		if [ -f composer.lock ]; then cp composer.lock /tmp/lock/; fi; \
		cd /tmp/lock; \
		composer update --no-install --no-scripts --minimal-changes; \
		cp composer.lock /repo/sdk/php/composer.lock'

_sdk-image-php:
	@echo "== sdk php image (PHP $(PHP_PATCH)) =="
	@docker build -q --build-arg PHP_PATCH=$(PHP_PATCH) -t $(PHP_IMAGE) sdk/php >/dev/null

# Every step runs on a copy of the checkout under /tmp with the image's vendor/
# copied in, so nothing lands in the mounted checkout: no vendor/, build/,
# .phpunit.cache or .php-cs-fixer.cache owned by root. vendor/ is copied
# rather than linked because Composer's autoloader finds the package's own
# src/ beside the real path of vendor/. The copy sits at /tmp/repo/sdk/php
# beside a link to the engine, so the tests read engine/Cargo.toml at the path
# they would in the checkout. Whatever a host install left in the copy
# (vendor/, build/, the caches) is removed first.
PHP_CHECKOUT := mkdir -p /tmp/repo/sdk; \
	cp -r /repo/sdk/php /tmp/repo/sdk/php; \
	ln -s /repo/engine /tmp/repo/engine; \
	cd /tmp/repo/sdk/php; \
	rm -rf vendor build .phpunit.cache .php-cs-fixer.cache; \
	cp -r /pkg/vendor vendor
# One command per step under `sh -e`, so the first red one stops the rest.
# `composer validate --strict` also stops a composer.lock older than a
# composer.json edit.
PHP_LINT := vendor/bin/php-cs-fixer check --diff; \
	vendor/bin/phpstan analyse --no-progress --memory-limit=512M; \
	composer validate --strict
# PHPUnit has no fail-under flag, so the 100% bar is read from its clover
# report.
PHP_TEST := vendor/bin/phpunit; php script/coverage-gate.php build/clover.xml

# Lint, the tests at 100% line coverage, then the package as a user gets it:
# archived by `composer archive` from a copy of the checkout with the
# workspace's license files beside it and the version engine/Cargo.toml names
# (Packagist takes it from the tag instead), the zip installed from an
# `artifact` repository with packagist.org turned off into a project of its
# own (script/check-installed-package, copied out of the checkout too, with
# whatever a host install left in it removed), and loaded from there on the
# line's PHP.
sdk\:verify\:php: ## Verify sdk/php on every supported line — php-cs-fixer + PHPStan + PHPUnit at 100% line coverage + composer archive/install/load (LATEST=1, PHP_VER=<line>)
	@$(call gate,_sdk-verify-php,sdk:verify:php)

_sdk-verify-php: _engine-cli-bin
	@$(call php_each,_sdk-verify-php-line)

_sdk-verify-php-line: _sdk-image-php
	@echo "== sdk php lint + test (PHP $(PHP_PATCH)) =="
	$(PHP_IN_DOCKER) '$(PHP_CHECKOUT); $(PHP_LINT); $(PHP_TEST)'
	@echo "== sdk php package (archive, install, load) (PHP $(PHP_PATCH)) =="
	$(PHP_IN_DOCKER) 'cp -r /repo/sdk/php /tmp/build; \
		cd /tmp/build; \
		rm -rf vendor build .phpunit.cache .php-cs-fixer.cache; \
		cp /repo/LICENSE-APACHE /repo/LICENSE-BSD /repo/LICENSE-MIT .; \
		version=$$(sed -n "/^\[workspace\.package\]/,/^\[/s/^version = \"\(.*\)\"/\1/p" /repo/engine/Cargo.toml); \
		[ -n "$$version" ] || { echo "no [workspace.package] version in engine/Cargo.toml" >&2; exit 1; }; \
		composer config version "$$version"; \
		composer archive --format=zip --dir=/tmp/pack --file=xlsx-graft; \
		cp -r /repo/sdk/php/script/check-installed-package /tmp/user; \
		cd /tmp/user; \
		rm -rf vendor composer.lock; \
		composer install --no-progress; \
		php check.php /tmp/user /tmp/pack/xlsx-graft.zip /repo/engine/Cargo.toml $(PHP_PATCH)'

sdk\:test\:php: ## sdk/php PHPUnit at 100% line coverage, on every supported line (LATEST=1, PHP_VER=<line>)
	@$(call gate,_sdk-test-php,sdk:test:php)

_sdk-test-php: _engine-cli-bin
	@$(call php_each,_sdk-test-php-line)

_sdk-test-php-line: _sdk-image-php
	@echo "== sdk php test (PHP $(PHP_PATCH)) =="
	$(PHP_IN_DOCKER) '$(PHP_CHECKOUT); $(PHP_TEST)'

# No engine CLI needed, so no cli-bin: lint runs without a Rust build.
sdk\:lint\:php: ## sdk/php php-cs-fixer, PHPStan (level max) and composer validate, on every supported line (LATEST=1, PHP_VER=<line>)
	@$(call gate,_sdk-lint-php,sdk:lint:php)

_sdk-lint-php:
	@$(call php_each,_sdk-lint-php-line)

_sdk-lint-php-line: _sdk-image-php
	@echo "== sdk php lint (PHP $(PHP_PATCH)) =="
	$(PHP_IN_DOCKER) '$(PHP_CHECKOUT); $(PHP_LINT)'
