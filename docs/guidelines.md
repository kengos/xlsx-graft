# Engineering Guidelines

> **Audience: contributors** (human or AI) working on the codebase.

These rules apply to every change. Where a rule can be checked by a tool, a
tool checks it — `make engine:verify` is the bar, and this page explains what
it enforces and why, plus the few rules no tool can see.

## Formatting and style: owned by the tools, not by review

- `rustfmt` formats and `clippy` lints. Do not hand-enforce style in review
  beyond what they check. If a tool's output looks wrong, change the tool's
  config file (`engine/rustfmt.toml`, `engine/clippy.toml`, the
  `[workspace.lints]` table in `engine/Cargo.toml`) — never add a parallel
  rule in a doc.
- Every deviation from a tool's defaults carries its reason as a comment in
  that config file, not elsewhere.
- A silenced lint says why in the attribute itself:
  `#[allow(clippy::x, reason = "...")]`.
  `clippy::allow_attributes_without_reason` gates `#[allow]` only; the same
  `reason` on `#[expect]` is checked in review.

## File length: a signal about the design, not a filing quota

Rust in this repository tends to grow long files, so length is capped and the
cap is gated:

| Scope | Per file | Per function | Gate |
| --- | --- | --- | --- |
| `engine/**/*.rs` | ≤300 lines, test files excluded; ≤160 is the design target for a NEW file, and is not gated | `clippy::too_many_lines`, 150 — test functions included | `make engine:budget`, `make engine:clippy` |

The cap exists for two reasons. An over-long file gets read wrong — by a
person skimming it, and more sharply by an AI, which reads an excerpt and
generalises from it: a file whose behaviour is decided near its end is
summarised from its beginning. And length is a smell: a file that keeps
growing has usually taken on a second job.

Both reasons are about the design rather than the arithmetic, so:

- **Split at a seam that means something, and the cap always wins.** When a
  cohesive unit will not fit, too much is being threaded through one place —
  fix that. Never raise the cap.
- **Never shave comments, blank lines or neighbouring tests to get under.**
  That satisfies the number and defeats both reasons for it.
- **`line-budget-exempt: <reason>`** (an in-file waiver) is for data tables,
  not logic.

### How to split

- **Directory module, no `mod.rs`**: `foo.rs` keeps the shared types and
  state, and each concern moves to `foo/<concern>.rs`.
  `clippy::mod_module_files` refuses `mod.rs`.
- **Keep the public paths stable**: items shared between the submodules are
  `pub(super)` or `pub(crate)`, never `pub`; the root `pub use`s what callers
  need, so a split never changes a caller's import.
- **Mirror the shape the neighbouring code already uses** instead of inventing
  a decomposition per file.
- **Plan the split before writing.** Count the lines (`wc -l`) of every file a
  change will touch — including the module root that new state or a new field
  implies — and put the split in the plan, not in a later cleanup.
- **Run `make engine:budget` after each step that adds code or tests**; it
  takes seconds, and finding the overrun inside `make engine:verify` costs the
  whole run.

## Where tests live, and what says which code they cover

Test files are outside the per-file budget (the 150-line function cap still
applies to them): a suite is a list of independent cases rather than a unit
of design, and splitting it to fit a number moves cases across a seam chosen
by arithmetic. What keeps a suite navigable is this:

- **Unit tests live in a sibling file, not inline**: `#[cfg(test)] mod tests;`
  beside the module (`foo/tests.rs`), not the Book's inline `mod tests { … }`.
- **When a suite grows, the DIRECTORY names the target and the FILE names the
  aspect** (`foo/tests/<aspect>.rs`). A module with a second suite but no
  `tests/` directory writes it as `foo/<aspect>_tests.rs`. Do not chase a
  one-to-one mapping between source files and test files.
- **Every split suite's `//!` header says what it covers** — as a behaviour
  when the suite drives a public entry point (most do), and as a source module
  only when it is genuinely scoped to one.
- **Near-e2e suites** (public API only) live in the crate's `tests/` directory
  as a single binary (`tests/<name>/main.rs` + modules). Clippy exempts only
  `#[test]` functions and `#[cfg(test)]` modules from the no-panic lints, not
  the helpers such a binary shares between cases, so its `main.rs` opens with:

  ```rust
  //! <what the suite covers>
  #![allow(
      clippy::unwrap_used,
      clippy::expect_used,
      clippy::panic,
      reason = "a test binary: a panic is how a failing case reports"
  )]
  ```

## Test coverage: 100% lines, over the whole workspace

- `make engine:coverage` fails below 100% line coverage, measured over the
  whole workspace. Never lower the threshold to make it pass.
- No exclusion markers, including `#[cfg(not(coverage))]` (`cfg(coverage)` is
  declared only to keep `unexpected_cfgs` quiet). An uncovered line is a
  finding with two honest answers: delete it (it is dead), or restructure so
  a test can reach the rule it encodes. The one exception is the next item.
- **The Node.js addon's N-API shim is outside the coverage gate**
  (`engine/napi/src/shim.rs`). It sits behind the `shim` feature, which the
  coverage run does not build, so llvm-cov never sees it; the JS SDK's tests
  run it through the real addon (`make engine:napi` builds it, and
  `engine:clippy` lints it with `--all-features`). That only holds while the
  shim maps an owned result to a JavaScript object and nothing more: a rule
  that lives in the shim is a rule no Rust test covers, so it goes in
  `engine/napi/src/call.rs`, which the coverage run does see.
- Coverage counts each crate twice (its own unit tests + the copy linked into
  dependents' tests); a line covered in only one copy can still fail. Cover
  new code in the crate's own unit tests first. `make engine:coverage-why`
  names the lines.
- 100% line coverage proves a line ran, not that a test asserts the right
  thing. A test whose title names a state must assert that state.

## Rust code rules

- **Every crate's `Cargo.toml` carries `[lints] workspace = true`**, written
  as a `[lints]` table with `workspace = true` under it (gated by
  `make engine:budget`). Without it none of the lints named on this page apply
  to that crate, and clippy stays green. A crate that must differ (the C ABI
  crate, the Node.js addon crate) overrides at its crate root with a
  `reason`, never by dropping the line — cargo does not accept crate-level
  lint entries beside `workspace = true`.
- **Every `.rs` file opens with a `//!` role header**, so `head -1` identifies
  it without opening it (gated by `make engine:budget`). **Every item
  reachable from a crate's public API has a doc comment** (`missing_docs`).
- **Errors are a `thiserror` enum per crate.** Do not leak ad-hoc error types
  through a public API.
- **No `unwrap()`, `expect()` or `panic!()` outside tests, and no `todo!()` or
  `unimplemented!()` anywhere** (`clippy::unwrap_used` / `expect_used` /
  `panic` / `todo` / `unimplemented`). A binary's `main.rs` stays
  thin; the logic it runs lives in the crate's library and follows the same
  rule.
- **Dependencies are declared once** in `[workspace.dependencies]` and
  referenced with `{ workspace = true }`. Every new dependency needs a reason;
  prefer pure Rust. Review `engine/Cargo.lock` diffs — an unexplained new
  transitive dependency is a finding.
- **Follow the neighbouring crate's idiom** (error enum shape, test layout)
  before inventing a new one.
- **Implementing to an external spec** (ECMA-376 / OOXML, Excel's own
  behaviour): read the primary source, never transcribe from memory.

## Untrusted input: every workbook is hostile

A workbook comes from someone else. Reading, verifying or editing one must
never crash the process or let its size decide how much memory we use.

- **Degrade to a diagnostic or an `Err`, never a panic.** The lints above stop
  `unwrap`/`expect`/`panic!`; review has to catch the rest reachable from
  parsed input — `unreachable!`, `assert!`, slice indexing, integer casts.
- **Bound everything the input controls**: entry counts, sizes, nesting depth,
  recursion. A new recursive walk over parsed structure needs an explicit
  bound. Test AT the bound's largest admitted value, not only past it, and
  assert the degraded output, not just "no panic".
- **Arithmetic on parsed integers uses `checked_*` / `saturating_*`**, never a
  bare `+`: release builds wrap silently, and a wrapped sum can defeat the
  guard it feeds. Negate or take `abs` in the widened type.
- **Diagnostics never echo unbounded input** (a cell value, a part name)
  without truncating it.

## `unsafe`

The workspace denies `unsafe_code`. The expected exceptions are the C ABI
crate, where raw pointers are unavoidable, and the Node.js addon crate
(`engine/napi`), which calls the C ABI's entry points; a crate that allows
`unsafe` must:

- deny `unsafe_op_in_unsafe_fn` and `clippy::undocumented_unsafe_blocks` at
  its root, so every block states why it is sound;
- confine raw-pointer handling to its marshalling modules, with the deciding
  logic in safe Rust;
- stop panics at the boundary (`catch_unwind` at every entry point; never
  `panic = "abort"`);
- state the caller's unchecked obligations (buffer lengths, freed handles)
  where the caller reads them: the C header for the C ABI crate. The addon
  crate leaves JavaScript no pointer to get wrong, so it has none; its own
  obligations to the C ABI are the `# Safety` sections in its `call` module.
