# xlsx-graft

> AI-only: token-dense repo map for coding agents; humans start at [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

A Rust engine that reads, verifies, creates and edits xlsx workbooks, served to
SDKs, a CLI and MCP from one engine.

Rust workspace lives in `engine/` (crates named `xlsx-graft-<dir>`).

## Rulebooks

| You need | Read |
| --- | --- |
| how to write and split Rust here: file length, module splits, test placement, coverage, errors, untrusted input, `unsafe` | [docs/guidelines.md](docs/guidelines.md) — read it before writing engine code |
| the commands that check anything | the comment block at the top of the [Makefile](Makefile); `make help` |

## Rules that bite

- No local cargo: the host has no Rust toolchain on purpose. Every check runs
  through `make` (Docker). **A correctness claim names the make target that
  said so and its PASS line** — never a hand-built `docker run … cargo …`
  equivalent. If the command you need is missing, add a target rather than
  typing a one-off.
- Each gate target (`engine:verify` / `lint` / `budget` / `fmt` / `clippy` /
  `test` / `coverage`) prints ONE PASS/FAIL line, exits 0 on PASS and non-zero
  on FAIL (the gate's own code is in the FAIL line and the `# exit` header of
  `last-error.log`), and keeps the full log in `.make-logs/`. A gate failure
  (without `V=1`) always lands at `.make-logs/last-error.log`. `V=1` streams
  the raw output instead (no log). The fix-applying targets (`engine:format` /
  `lock` / `update`) print their raw output.
- **Never wrap a gate** in a pipe (`| tail`), in `; echo $?`, or in `make -n`.
  The first two report the wrong exit code; the Makefile refuses the third.
- GNU Make 4 or newer. The Makefile refuses 3.81 (macOS's) and names the fix.
- Every gate is `--locked`: after a `Cargo.toml` edit run `make engine:lock`
  and commit `engine/Cargo.lock`.
- `make engine:verify` is the engine's bar: line budget + `//!` headers, fmt,
  clippy `-D warnings` (with the workspace lints in `engine/Cargo.toml`), tests
  at **100% line coverage over the whole workspace**, and doctests.
- **Rust here grows long files, so length is gated**: ≤300 lines per non-test
  file (aim for ≤160 in a new one; not gated), 150 per function. Before
  editing, `wc -l` the files you will touch and plan the split (`foo.rs` +
  `foo/<concern>.rs`, no `mod.rs`); never trim comments or tests to fit.
  Details: [docs/guidelines.md](docs/guidelines.md) § File length.
- No `unwrap` / `expect` / `panic!` outside tests, no `todo!` anywhere: every
  workbook is untrusted input and degrades to a diagnostic or an `Err`. A
  silenced lint carries `reason = "..."`.
- Plans and roadmap are not in this repository.
