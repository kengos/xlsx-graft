# sdk/dotnet — the .NET SDK's gates and lockfiles (mk/sdk.mk names the values
# a language's file sets and what `sdk_lang` makes of them)

.PHONY: _sdk-image-dotnet _sdk-lint-dotnet _sdk-lint-dotnet-line _sdk-test-dotnet \
        _sdk-test-dotnet-line _sdk-verify-dotnet _sdk-verify-dotnet-line sdk\:lint\:dotnet \
        sdk\:lock\:dotnet sdk\:test\:dotnet sdk\:verify\:dotnet

# The floor first and the latest stable line last: LATEST=1 takes the last and
# sdk:lock:dotnet the first. Each line is pinned to an SDK release, not to a
# runtime one: the SDK release names the image, and it is the SDK that decides what `dotnet format` and the analyzers check. The base
# image's Ubuntu layers still move when Microsoft rebuilds the tag.
DOTNET_LINES := 10.0
DOTNET_PATCH_10.0 := 10.0.402
# Command line only (Makefile § knobs).
DOTNET_VER :=
DOTNET_IMAGE_INPUTS := $(addprefix sdk/dotnet/,Dockerfile Directory.Build.props \
                         XlsxGraft/XlsxGraft.csproj XlsxGraft/packages.lock.json \
                         XlsxGraft.Tests/XlsxGraft.Tests.csproj XlsxGraft.Tests/packages.lock.json)
$(eval $(call sdk_lang,dotnet,DOTNET))

# Writes both projects' packages.lock.json (restoring the tests restores the
# library they reference) and does NOT go through `quiet`, like engine:lock. It
# runs on the Dockerfile's `sdk` stage, the .NET SDK without the restore: the
# gate image restores from the lockfiles, so it cannot be the one that writes
# them. A lockfile with no RID in it holds for every platform. It runs on the
# floor only as sdk:lock:ruby does; whether every line restores the lock is
# what that line's gate checks.
sdk\:lock\:dotnet: override DOTNET_VER = $(firstword $(DOTNET_LINES))
sdk\:lock\:dotnet: ## Re-resolve sdk/dotnet's packages.lock.json files after a csproj edit (then: git add them)
	@echo "== sdk:lock:dotnet (.NET SDK $(DOTNET_PATCH)) =="
	docker build -q --target sdk --build-arg DOTNET_VERSION=$(DOTNET_PATCH) \
		-t $(DOTNET_IMAGE)-sdk sdk/dotnet >/dev/null
	$(GATE_LOCK) docker run --rm --init --label "xlsx-graft.tree=$(CURDIR)" \
		-v "$(CURDIR):/repo" -w /repo/sdk/dotnet $(DOTNET_IMAGE)-sdk \
		dotnet restore --force-evaluate XlsxGraft.Tests/XlsxGraft.Tests.csproj

_sdk-image-dotnet:
	@echo "== sdk dotnet image (.NET SDK $(DOTNET_PATCH)) =="
	@docker build -q --build-arg DOTNET_VERSION=$(DOTNET_PATCH) -t $(DOTNET_IMAGE) sdk/dotnet >/dev/null

# Lint, the tests at 100% line coverage, then the package as a user gets it:
# packed from a copy of the checkout, restored into a project of its own
# (script/CheckInstalledPackage, copied out of the checkout too) from the
# packed nupkg alone and into a package cache of its own, and run there.
sdk\:verify\:dotnet: ## Verify sdk/dotnet on every supported line — dotnet format + analyzers + xUnit at 100% line coverage + nupkg pack/install/load (LATEST=1, DOTNET_VER=<line>)
	@$(call gate,_sdk-verify-dotnet,sdk:verify:dotnet)

_sdk-verify-dotnet: _engine-capi-lib
	@$(call dotnet_each,_sdk-verify-dotnet-line)

_sdk-verify-dotnet-line: _sdk-image-dotnet
	@echo "== sdk dotnet lint + test (.NET SDK $(DOTNET_PATCH)) =="
	$(DOTNET_IN_DOCKER) '$(DOTNET_LINT); $(DOTNET_TEST)'
	@echo "== sdk dotnet nupkg (pack, install, load) (.NET SDK $(DOTNET_PATCH)) =="
	$(DOTNET_IN_DOCKER) 'cp -r /repo/sdk/dotnet /tmp/build; \
		dotnet pack /tmp/build/XlsxGraft/XlsxGraft.csproj -c Release -o /tmp/pkg; \
		cp -r /repo/sdk/dotnet/script/CheckInstalledPackage /tmp/user; \
		cp /repo/sdk/dotnet/.editorconfig /tmp/user/; \
		cd /tmp/user; \
		NUGET_PACKAGES=/tmp/user-packages dotnet run -c Release -- /tmp/user-packages /repo/engine/Cargo.toml'

sdk\:test\:dotnet: ## sdk/dotnet xUnit at 100% line coverage, on every supported line (LATEST=1, DOTNET_VER=<line>)
	@$(call gate,_sdk-test-dotnet,sdk:test:dotnet)

_sdk-test-dotnet: _engine-capi-lib
	@$(call dotnet_each,_sdk-test-dotnet-line)

_sdk-test-dotnet-line: _sdk-image-dotnet
	@echo "== sdk dotnet test (.NET SDK $(DOTNET_PATCH)) =="
	$(DOTNET_IN_DOCKER) '$(DOTNET_TEST)'

# One command per step under `sh -e`, so the first red one stops the rest.
# `dotnet format` skips a referenced project, so each project is formatted on
# its own; building the tests builds the library. The check script cannot be
# restored before the pack, so it gets the whitespace check here and its
# analyzers, under the same .editorconfig, when sdk:verify:dotnet builds it.
DOTNET_LINT := dotnet format XlsxGraft/XlsxGraft.csproj --verify-no-changes; \
	dotnet format XlsxGraft.Tests/XlsxGraft.Tests.csproj --verify-no-changes; \
	dotnet format whitespace script/CheckInstalledPackage --folder --verify-no-changes; \
	dotnet build XlsxGraft.Tests/XlsxGraft.Tests.csproj -c Release
DOTNET_TEST := dotnet test XlsxGraft.Tests/XlsxGraft.Tests.csproj

# No engine library needed, so no capi-lib: lint runs without a Rust build.
sdk\:lint\:dotnet: ## sdk/dotnet dotnet format and the analyzers (warnings as errors; script/ whitespace only), on every supported line (LATEST=1, DOTNET_VER=<line>)
	@$(call gate,_sdk-lint-dotnet,sdk:lint:dotnet)

_sdk-lint-dotnet:
	@$(call dotnet_each,_sdk-lint-dotnet-line)

_sdk-lint-dotnet-line: _sdk-image-dotnet
	@echo "== sdk dotnet lint (.NET SDK $(DOTNET_PATCH)) =="
	$(DOTNET_IN_DOCKER) '$(DOTNET_LINT)'
