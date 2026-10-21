# Contributing

## What you need

- Docker. Every check runs in a container, so you do not need a Rust toolchain
  on the host.
- GNU Make 4 or newer. macOS ships 3.81, which the Makefile refuses. Install
  Homebrew's make and put it first on `PATH`:

  ```sh
  export PATH="$(brew --prefix make)/libexec/gnubin:$PATH"
  ```

## Checking a change

```sh
make engine:verify
```

It runs the line budget, `cargo fmt --check`, `cargo clippy -D warnings`, the
tests at 100% line coverage, and the doctests. It prints one PASS/FAIL line; on
a failure the full log is at `.make-logs/last-error.log`. Add `V=1` to see the
raw output.

`make help` lists the faster slices (`engine:fmt`, `engine:clippy`,
`engine:test`, …) and the commands that fix things (`engine:format`,
`engine:lock`).

The first run downloads the toolchain image and installs `cargo-llvm-cov`
into Docker volumes named after the Rust version
(`xlsx-graft-cargo-<version>`, `xlsx-graft-rustup-<version>`); later runs
reuse them. `make clean:cache` drops the current version's volumes.
