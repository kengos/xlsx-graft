# sdk/jvm — the Java SDK's gates and lockfiles (mk/sdk.mk names the values a
# language's file sets and what `sdk_lang` makes of them)

.PHONY: _sdk-image-jvm _sdk-lint-jvm _sdk-lint-jvm-line _sdk-test-jvm \
        _sdk-test-jvm-line _sdk-verify-jvm _sdk-verify-jvm-line sdk\:lint\:jvm \
        sdk\:lock\:jvm sdk\:test\:jvm sdk\:verify\:jvm

# The floor first and the latest stable line last: LATEST=1 takes the last and
# sdk:lock:jvm the first. Only LTS lines: a non-LTS line lives six months, so
# it is always within six months of its end (docs/sdk.md). Each line is pinned
# to the Temurin release of the JDK its tests run on, as Docker Hub tags it
# (`_` where the version has `+`). The JDK that runs Gradle and the tools is
# pinned in sdk/jvm/Dockerfile. The base image's Ubuntu layers still move when
# Docker Hub rebuilds the tag.
JVM_LINES := 8 11 17 21 25
JVM_PATCH_8 := 8u504-b01
JVM_PATCH_11 := 11.0.32.1_1
JVM_PATCH_17 := 17.0.20.1_1
JVM_PATCH_21 := 21.0.12.1_1
JVM_PATCH_25 := 25.0.4.1_1
# Command line only (Makefile § knobs).
JVM_VER :=
JVM_IMAGE_INPUTS := $(addprefix sdk/jvm/,Dockerfile settings.gradle.kts build.gradle.kts \
                      gradle.properties gradle.lockfile buildscript-gradle.lockfile)
$(eval $(call sdk_lang,jvm,JVM))

# Writes sdk/jvm/gradle.lockfile and buildscript-gradle.lockfile (the plugins)
# and does NOT go through `quiet`, like engine:lock. It runs on the
# Dockerfile's `tools` stage, the JDKs and Gradle without the dependencies:
# the gate image resolves from the lockfiles, so it cannot be the one that
# writes them. JVM dependencies do not vary by platform, so one lockfile holds
# for x86-64 and arm64. Gradle resolves on the tools' JDK whatever the line;
# the floor only names the stage's line JDK. Whether every line runs on the
# lock is what that line's gate checks.
sdk\:lock\:jvm: override JVM_VER = $(firstword $(JVM_LINES))
sdk\:lock\:jvm: ## Re-resolve sdk/jvm's gradle.lockfile and buildscript-gradle.lockfile after a build file edit (then: git add them)
	@echo "== sdk:lock:jvm (Temurin $(JVM_PATCH)) =="
	docker build -q --target tools --build-arg JVM_PATCH=$(JVM_PATCH) --build-arg JVM_LINE=$(JVM_VER) \
		-t $(JVM_IMAGE)-tools sdk/jvm >/dev/null
	$(GATE_LOCK) docker run --rm --init --label "xlsx-graft.tree=$(CURDIR)" \
		-v "$(CURDIR):/repo" -w /repo/sdk/jvm $(JVM_IMAGE)-tools \
		gradle --project-cache-dir /tmp/gradle-project dependencies --write-locks

_sdk-image-jvm:
	@echo "== sdk jvm image (Temurin $(JVM_PATCH)) =="
	@docker build -q --build-arg JVM_PATCH=$(JVM_PATCH) --build-arg JVM_LINE=$(JVM_VER) \
		-t $(JVM_IMAGE) sdk/jvm >/dev/null

# Lint, the tests at 100% line coverage, then the jar as a user gets it:
# published from a copy of the checkout into a Maven repository of its own,
# resolved from there (and the image's cache, for JNA) by a build of its own
# (script/check-installed-jar, copied out of the checkout too), and run there
# on the line's JVM.
sdk\:verify\:jvm: ## Verify sdk/jvm on every supported line — Spotless + javac lint + javadoc + JUnit at 100% line coverage + jar publish/install/load (LATEST=1, JVM_VER=<line>)
	@$(call gate,_sdk-verify-jvm,sdk:verify:jvm)

_sdk-verify-jvm: _engine-capi-lib
	@$(call jvm_each,_sdk-verify-jvm-line)

_sdk-verify-jvm-line: _sdk-image-jvm
	@echo "== sdk jvm lint + test (Temurin $(JVM_PATCH)) =="
	$(JVM_IN_DOCKER) '$(JVM_GRADLE) $(JVM_LINT) $(JVM_TEST)'
	@echo "== sdk jvm jar (publish, install, load) (Temurin $(JVM_PATCH)) =="
	$(JVM_IN_DOCKER) 'cp -r /repo/sdk/jvm /tmp/build; \
		cd /tmp/build; \
		gradle --offline publishToMavenLocal -Dmaven.repo.local=/tmp/repo; \
		cp -r /repo/sdk/jvm/script/check-installed-jar /tmp/user; \
		cd /tmp/user; \
		gradle --offline run --args="/tmp/repo /repo/sdk/jvm/gradle.lockfile /repo/engine/Cargo.toml"'

sdk\:test\:jvm: ## sdk/jvm JUnit at 100% line coverage, on every supported line's JVM (LATEST=1, JVM_VER=<line>)
	@$(call gate,_sdk-test-jvm,sdk:test:jvm)

_sdk-test-jvm: _engine-capi-lib
	@$(call jvm_each,_sdk-test-jvm-line)

_sdk-test-jvm-line: _sdk-image-jvm
	@echo "== sdk jvm test (Temurin $(JVM_PATCH)) =="
	$(JVM_IN_DOCKER) '$(JVM_GRADLE) $(JVM_TEST)'

# One Gradle run takes the tasks in the order given and stops at the first red
# one. .gradle/ goes to /tmp, out of the mounted checkout.
JVM_GRADLE := gradle --offline --project-cache-dir /tmp/gradle-project
JVM_LINT := spotlessCheck compileJava compileTestJava javadoc
JVM_TEST := test jacocoTestCoverageVerification

# No engine library needed, so no capi-lib: lint runs without a Rust build.
# Every lint task runs on the tools' JDK, so each line lints the same; it runs
# per line all the same, as every language's lint does.
sdk\:lint\:jvm: ## sdk/jvm Spotless (google-java-format), javac -Xlint:all -Werror and javadoc doclint, on every supported line (LATEST=1, JVM_VER=<line>)
	@$(call gate,_sdk-lint-jvm,sdk:lint:jvm)

_sdk-lint-jvm:
	@$(call jvm_each,_sdk-lint-jvm-line)

_sdk-lint-jvm-line: _sdk-image-jvm
	@echo "== sdk jvm lint (Temurin $(JVM_PATCH)) =="
	$(JVM_IN_DOCKER) '$(JVM_GRADLE) $(JVM_LINT)'
