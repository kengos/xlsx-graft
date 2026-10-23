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

It runs every part's checks; today that is `make engine:verify` and
`make sdk:verify`, every SDK on every language version it supports. While
iterating, `make verify LATEST=1` runs each SDK on its latest version only,
which is faster; run the whole `make verify` before asking for a review.

`make engine:verify` is the engine's: the line budget, the check that every
crate opts into the workspace lints, the check that the toolchain versions in
the `Makefile` and in `docker/toolchain.Dockerfile` agree and that the pinned
image holds the versions its tag names, `cargo fmt --check`,
`cargo clippy -D warnings`, the tests at 100% line coverage, the doctests, and
the check that the C ABI library exports exactly the `xlsx_graft_*` functions
its header declares, and nothing else. It prints one PASS/FAIL line; on a failure the
full log is at `.make-logs/last-error.log`. Add `V=1` to see the raw output.

`make sdk:verify:ruby` is the Ruby SDK's, on every supported Ruby (`RUBY_LINES`
in `mk/sdk/ruby.mk`): rubocop, the specs at 100% line coverage, and the gem built,
installed and loaded against the engine library. `LATEST=1` runs it on the
newest Ruby only, `RUBY_VER=<line>` on that one.

`make sdk:verify:python` is the Python SDK's, on every supported Python
(`PYTHON_LINES` in `mk/sdk/python.mk`): ruff, mypy in strict mode, the tests at
100% line coverage, and the wheel built, installed and loaded against the
engine library. `LATEST=1` runs it on the newest Python only,
`PYTHON_VER=<line>` on that one.

[docs/sdk.md](docs/sdk.md) has the shape every SDK follows.

CI runs what `make verify` runs, in the same toolchain image, on every pull
request and every push to `main`
([.github/workflows/ci.yml](.github/workflows/ci.yml)). It splits it into
parallel jobs, the engine and one per supported Ruby and Python line, which
`make ci:matrix` lists, and its `make verify` check passes only when all of
them do. It keeps no second copy of the checks; it only adds `V=1` for the raw
output.

`make help` lists the faster slices (`engine:fmt`, `engine:clippy`,
`engine:test`, `sdk:lint`, `sdk:test:ruby`, `sdk:test:python`, …) and the
commands that fix things (`engine:format`, `engine:lock`, `sdk:lock:ruby`,
`sdk:lock:python`).

The first run pulls the toolchain image (about 1 GB), which already holds
Rust, rustfmt, clippy and `cargo-llvm-cov`, and downloads the crates into the
Docker volume `xlsx-graft-cargo-cache`; later runs reuse both. The SDK gates
also pull their language's base image (`ruby:<version>-slim-bookworm`,
`python:<version>-slim-bookworm`) and build a gate image on it from
`sdk/<lang>/Dockerfile` (Ruby's apt packages and locked gems, Python's uv and
locked tools), which is never published. Python's image build also pulls the
pinned `ghcr.io/astral-sh/uv` image to copy `uv` from. Nothing else is
installed or built besides the engine itself, so `make docker:clean:all` is
safe at any time: the next run pulls, downloads and builds again. It removes
the SDK gate images but not the base images or the `uv` image, which other
projects may use. If nothing else does, remove the ones `docker images` lists
with `docker rmi`; the ones it does not list are only in Docker's build
cache, which `docker builder prune` empties (other projects' cache included).

If you ran the checks before the toolchain image, `make docker:clean:all` also
removes the volumes that layout left behind (`xlsx-graft-cargo-<version>`,
`xlsx-graft-rustup-<version>`). The `rust:<version>-slim-bookworm` image it
ran in is only listed, since other projects may use it; remove it with the
`docker rmi` printed next to it if nothing else does.
