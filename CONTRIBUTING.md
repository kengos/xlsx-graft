# Contributing

## What you need

- Docker. Every check runs in a container, so you do not need a Rust toolchain
  on the host.
- GNU Make 4 or newer. macOS ships 3.81, which the Makefile refuses. Install
  Homebrew's make and put it first on `PATH`:

  ```sh
  export PATH="$(brew --prefix make)/libexec/gnubin:$PATH"
  ```

## Writing code

Read [docs/guidelines.md](docs/guidelines.md) first: file length and how to
split a module, where tests go, the coverage bar, and the rules for untrusted
input. Most of it is enforced by the check below.

## Checking a change

```sh
make verify
```

It runs every part's checks; today that is `make engine:verify`, the
engine's: the line budget, the check that every crate opts into the workspace
lints, the check that the toolchain versions in the `Makefile` and in
`docker/toolchain.Dockerfile` agree and that the pinned image holds the
versions its tag names,
`cargo fmt --check`, `cargo clippy -D warnings`, the tests at 100% line
coverage, the doctests, and the check that the C ABI library exports exactly
the `xlsx_graft_*` functions its header declares, and nothing else. It prints
one PASS/FAIL line; on a failure the full log is at
`.make-logs/last-error.log`. Add `V=1` to see the raw output.

`make help` lists the faster slices (`engine:fmt`, `engine:clippy`,
`engine:test`, …) and the commands that fix things (`engine:format`,
`engine:lock`).

The first run pulls the toolchain image (about 1 GB), which already holds
Rust, rustfmt, clippy and `cargo-llvm-cov`, and downloads the crates into the
Docker volume `xlsx-graft-cargo-cache`; later runs reuse both. Nothing is
installed or built besides the engine itself, so `make docker:clean:all` is
safe at any time: the next run pulls and downloads again.

If you ran the checks before the toolchain image, `make docker:clean:all` also
removes the volumes that layout left behind (`xlsx-graft-cargo-<version>`,
`xlsx-graft-rustup-<version>`). The `rust:<version>-slim-bookworm` image it
ran in is only listed, since other projects may use it; remove it with the
`docker rmi` printed next to it if nothing else does.
