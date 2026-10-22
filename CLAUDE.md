# xlsx-graft

> AI-only: token-dense repo map for coding agents; humans start at [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

A Rust engine that reads, verifies, creates and edits xlsx workbooks, served to
SDKs, a CLI and MCP from one engine.

Rust workspace lives in `engine/` (crates named `xlsx-graft-<dir>`).

## Rulebooks

| You need | Read |
| --- | --- |
| how to write and split Rust here: file length, module splits, test placement, coverage, errors, untrusted input, `unsafe` | [docs/guidelines.md](docs/guidelines.md) — read it before writing engine code |
| adding or changing a language SDK: supported versions, the `sdk:*:<lang>` gates, finding the engine library, calling the C ABI | [docs/sdk.md](docs/sdk.md) |
| the commands that check anything | the comment block at the top of the [Makefile](Makefile); `make help` |

## Rules that bite

- No local cargo: the host has no Rust toolchain on purpose. Every check runs
  through `make` (Docker). **A correctness claim names the make target that
  said so and its PASS line** — never a hand-built `docker run … cargo …`
  equivalent. A PASS line names the command that ran, variables included
  (not `V=` / `JOBS=` / `CARGO_VOLUME=`, which change how a gate runs, not
  what it checks): one carrying `LATEST=1`, `RUBY_VER=`, `P=` or `F=` is a
  narrowed run, and the bar is the bare name. If the command you need is
  missing, add a target rather than typing a one-off.
- Each gate target (`verify`, `engine:verify` / `lint` / `budget` /
  `fmt` / `clippy` / `test` / `coverage` / `capi-abi` / `capi-lib`, and
  `sdk:verify` / `test` / `lint`, each also `:ruby`) prints ONE PASS/FAIL
  line, exits 0 on PASS and non-zero on FAIL (the gate's own code is in the
  FAIL line and the `# exit` header of `last-error.log`), and keeps the full
  log in `.make-logs/`. A gate failure (without `V=1`) always lands at
  `.make-logs/last-error.log`. `V=1` streams the raw output instead (no log).
  The fix-applying targets (`engine:format` / `lock` / `update`,
  `sdk:lock:ruby`) print their raw output.
- **Never wrap a gate** in a pipe (`| tail`), in `; echo $?`, or in `make -n`.
  The first two report the wrong exit code; the Makefile refuses the third.
- GNU Make 4 or newer. The Makefile refuses 3.81 (macOS's) and names the fix.
- Every gate is `--locked`: after a `Cargo.toml` edit run `make engine:lock`
  and commit `engine/Cargo.lock`.
- `make verify` is the whole bar: every scope's `<scope>:verify` (today
  `engine:verify` and `sdk:verify`), **every SDK on every supported line**,
  the floor included. `LATEST=1` narrows each SDK to its latest stable line
  for a faster local run; it is not the bar. CI runs the same members as
  parallel jobs with `V=1`, listed by `make ci:matrix` (one for the engine,
  one per SDK line), and its required `make verify` job passes only when all
  of them do (`.github/workflows/ci.yml`); a check CI needs is a make target,
  never a workflow step.
- `make engine:verify` is the engine's bar: line budget + `//!` headers + every
  crate's `[lints] workspace = true` + the toolchain pins (the Makefile's
  versions equal `docker/toolchain.Dockerfile`'s, and the pinned image holds
  the versions of its tag), fmt, clippy `-D warnings` (with the workspace
  lints in `engine/Cargo.toml`), tests at **100% line coverage over the whole
  workspace**, doctests, and the C ABI check (the built shared
  library's `xlsx_graft_*` exports match `engine/capi/include/xlsx_graft.h`).
- `make sdk:verify:ruby` is the Ruby SDK's bar on every line in `RUBY_LINES`
  (`mk/sdk.mk`, the floor first and the latest last; `LATEST=1` or
  `RUBY_VER=<line>` narrows it): rubocop + rspec at **100% line
  coverage** + the gem built, installed into its own `GEM_HOME` and loaded
  against `dist/capi/local/`, built first by `engine:capi-lib`.
- **Rust here grows long files, so length is gated**: ≤300 lines per non-test
  file (aim for ≤160 in a new one; not gated), 150 per function. Before
  editing, `wc -l` the files you will touch and plan the split (`foo.rs` +
  `foo/<concern>.rs`, no `mod.rs`); never trim comments or tests to fit.
  Details: [docs/guidelines.md](docs/guidelines.md) § File length.
- No `unwrap` / `expect` / `panic!` outside tests, no `todo!` anywhere: every
  workbook is untrusted input and degrades to a diagnostic or an `Err`. A
  silenced lint carries `reason = "..."`.
- Plans and roadmap are not in this repository.
