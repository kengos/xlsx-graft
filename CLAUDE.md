# xlsx-graft

> AI-only: token-dense repo map for coding agents; humans start at [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

A Rust engine that reads, verifies, creates and edits xlsx workbooks, served to
SDKs, a CLI and MCP from one engine.

Rust workspace lives in `engine/` (crates named `xlsx-graft-<dir>`). Every `.rs`
file opens with a `//!` role header, so `head -1` identifies a file without
opening it; unit tests live in `#[cfg(test)]`-only sibling files
(`<mod>/tests.rs`, further split under `<mod>/tests/`), near-e2e suites in the
crate's `tests/` directory.

## Rules that bite

- No local cargo: the host has no Rust toolchain on purpose. Every check runs
  through `make` (Docker). The sanctioned commands are the comment block at
  the top of the [Makefile](Makefile); `make help` lists every target. **A correctness claim names the make target that said so and its
  PASS line** — never a hand-built `docker run … cargo …` equivalent. If the
  command you need is missing, add a target rather than typing a one-off.
- Each gate target (`engine:verify` / `lint` / `budget` / `fmt` / `clippy` /
  `test` / `coverage`) prints ONE PASS/FAIL line, exits 0 on PASS and
  non-zero on FAIL (the gate's own code is in the FAIL line and the `# exit`
  header of `last-error.log`), and keeps the full log in `.make-logs/`. The fix-applying targets
  (`engine:format` / `lock` / `update`) print their raw output instead. A
  gate failure (without `V=1`) always lands at `.make-logs/last-error.log`. `V=1` streams the raw output instead (no log).
- **Never wrap a gate** in a pipe (`| tail`), in `; echo $?`, or in `make -n`.
  The first two report the wrong exit code; the Makefile refuses the third.
- GNU Make 4 or newer. The Makefile refuses 3.81 (macOS's) and names the fix.
- Every gate is `--locked`: after a `Cargo.toml` edit run `make engine:lock`
  and commit `engine/Cargo.lock`.
- `make engine:verify` is the engine's bar: line budget, fmt, clippy
  `-D warnings`, tests at **100% line coverage over the whole workspace**, and
  doctests.
- Line budget: every non-test `.rs` under `engine/` is ≤300 lines (≤160 is the
  design target for a new file); exceeding it needs an in-file
  `line-budget-exempt: <reason>` waiver. Hitting the cap means split at a seam
  that means something, not trim. `clippy::too_many_lines` (150,
  `engine/clippy.toml`) gates function length. Test files are outside the
  length budget.
- Coverage counts each crate twice (its own unit-test binary + the copy linked
  into dependents' test binaries); a line covered in only one copy can still
  fail the gate — cover new code in the crate's own unit tests first.
  `make engine:coverage-why` names the lines.
- Plans and roadmap are not in this repository.
