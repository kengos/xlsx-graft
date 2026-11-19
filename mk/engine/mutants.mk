# engine/ — mutation testing of the change since a base revision
#
# Public target `engine:mutants`, with the recipe in `_engine-mutants`, like
# mk/engine.mk. It is not a member of engine:verify (see its comment below).

.PHONY: _engine-mutants engine\:mutants

# The revision the change is measured from. A knob that changes WHAT the gate
# checks, so it is set on the command line only (`make engine:mutants
# BASE=origin/release`), with a bare `:=` like the other knobs. `origin/main`
# is the local copy: `git fetch` first, or the diff includes other people's
# changes.
BASE := origin/main

# cargo-mutants over the lines of engine/ that changed since BASE, the working
# tree included. Not a member of engine:verify or make verify: what it mutates
# changes with BASE and the working tree, and its time with the size of the
# diff, so it is a tool to run by hand before a commit, not a bar every commit
# must pass.
#
# Exit codes: the FAIL line always says `exit 2`, the code make itself ends
# with when a recipe fails. The N of `make[...]: *** [...] Error N` in the log
# is the real one:
# - 0: every mutant was caught or unviable.
# - 2, 3: cargo-mutants found a mutant missed (2) or timed out (3).
# - 4: cargo-mutants's own baseline failed, the tests of the crates it mutates
#   on an unmutated copy.
# - 1: one of this recipe's own checks failed: BASE is not a commit, an
#   untracked .rs file, or the baseline of the whole workspace (below).
# The results are in engine/target/mutants/mutants.out/ (caught.txt,
# missed.txt, timeout.txt, unviable.txt, outcomes.json).
#
# - The diff is taken by the git of the host, not the container's: the .git of
#   a worktree is a file that points outside the mounted directory.
# - The git options pin the diff to the format cargo-mutants reads, whatever
#   the git config of the host says: `--no-ext-diff` (a `diff.external` tool
#   writes another format), `--no-color`, `--src-prefix=a/ --dst-prefix=b/`
#   (`diff.mnemonicPrefix` makes the prefixes c/ and w/, which match no path),
#   `--no-renames` (a moved file is a deletion and an addition, so the whole
#   file at its new path is mutated, not an entry with no hunk). Tried with a
#   `git mv` of a .rs file: cargo-mutants 27.1.0 takes the deletion entry
#   (`+++ /dev/null`) without an error and mutates nothing for it.
# - The pathspec holds only .rs files, the only files cargo-mutants mutates; a
#   change to Cargo.lock or a .md is not passed to it, and a change with no
#   .rs file starts no Docker.
# - `--merge-base` compares the working tree with the common ancestor of BASE
#   and HEAD, so changes not yet committed are in the diff. An untracked .rs
#   file is not in it, so it fails the gate until `git add -N <file>`: passing
#   without mutating a new file would be a wrong PASS.
# - `--relative=engine` drops the `engine/` prefix, because cargo-mutants
#   matches the paths of the diff against paths relative to the workspace
#   (engine/).
# - cargo-mutants copies the workspace to /tmp/cargo-mutants-*/ and mutates the
#   copy, so a test's CARGO_MANIFEST_DIR/../../fixtures resolves to
#   /tmp/fixtures: the symlink points it at the mounted fixtures/. This holds
#   while TMPDIR is unset (the copy is in /tmp). `--in-place` would avoid the
#   copy but rewrites the mounted sources, and an interrupted run leaves the
#   rewrite in them.
# - `--test-workspace=true` runs the tests of the whole workspace for each
#   mutant. The coverage bar is 100% of the whole workspace, so a line that
#   only another crate's tests check is not reported as missed. Each mutant
#   takes longer. It does not widen cargo-mutants's own baseline, which runs
#   only the tests of the crates it mutates (cargo-mutants 27.1.0), so a
#   failing test of another crate would fail every mutant and pass as all
#   caught. This recipe therefore runs the tests of the whole workspace once
#   first, on a copy of engine/ at /tmp/baseline (the same depth as
#   cargo-mutants's copy, so ../../fixtures is the same symlink), and stops
#   with exit 1 if they fail. A test that reads another path outside engine/
#   (../../sdk, ...) fails there. The copy's target directory is
#   engine/target/mutants/baseline, because the copy's path is the same every
#   run, so the next run reuses the build of the dependencies.
# - `--minimum-test-timeout=120` fixes the floor of the test timeout.
#   cargo-mutants sets the timeout from the time of its own baseline (the tests
#   of the mutated crates only) and never below 20 seconds, but each mutant runs
#   the tests of the whole workspace, so a surviving mutant could exceed it and
#   be reported as a timeout, not as missed. The tests of the workspace take a
#   few seconds now, so 120 is ample; a mutant that loops forever costs 2
#   minutes.
# - JOBS caps the parallelism of cargo, as in the other gates; the mutants run
#   one at a time, cargo-mutants's default.
engine\:mutants: ## cargo-mutants over the engine/ .rs change since BASE (default origin/main), working tree included; not in engine:verify
	@$(call gate,_engine-mutants,engine:mutants)

_engine-mutants:
	@echo "== mutants (diff since $(BASE)) =="
	@git rev-parse --verify --quiet '$(BASE)^{commit}' >/dev/null || { \
		echo "BASE=$(BASE) is not a commit here (git fetch first?)"; exit 1; }
	@untracked=$$(git ls-files --others --exclude-standard -- ':(glob)engine/**/*.rs'); \
	if [ -n "$$untracked" ]; then \
		echo "$$untracked" | while read -r f; do \
			echo "$$f: untracked; \`git add -N $$f\` so the diff includes it"; done; \
		exit 1; fi
	@mkdir -p engine/target/mutants
	@git diff --no-ext-diff --no-color --no-renames --src-prefix=a/ --dst-prefix=b/ \
		--merge-base '$(BASE)' --relative=engine -- ':(glob)engine/**/*.rs' \
		> engine/target/mutants/in-diff.patch
	@if [ ! -s engine/target/mutants/in-diff.patch ]; then \
		echo "no .rs change under engine/ since $(BASE); nothing to mutate"; exit 0; fi; \
	$(CARGO_IN_DOCKER) 'ln -s /repo/fixtures /tmp/fixtures; \
		mkdir /tmp/baseline; \
		tar -C /repo/engine --exclude=./target -cf - . | tar -C /tmp/baseline -xf -; \
		(cd /tmp/baseline && CARGO_TARGET_DIR=/repo/engine/target/mutants/baseline \
			cargo test --workspace --locked $(CARGO_JOBS)) \
		|| { echo "engine:mutants: the workspace tests fail on an unmutated copy at /tmp/baseline; fix them before mutating" >&2; exit 1; }; \
		cd /repo/engine && cargo mutants \
			--in-diff target/mutants/in-diff.patch --output target/mutants \
			--test-workspace=true --minimum-test-timeout=120 \
			--cargo-arg=--locked $(if $(JOBS),--cargo-arg=--jobs=$(JOBS))' \
	|| { code=$$?; echo; if [ $$code = 2 ]; then \
		echo "-- surviving mutants: engine/target/mutants/mutants.out/missed.txt --"; \
		else echo "-- results: engine/target/mutants/mutants.out/ --"; fi; exit $$code; }; \
	echo "-- surviving mutants: engine/target/mutants/mutants.out/missed.txt --"
