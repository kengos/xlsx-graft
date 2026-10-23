# SDK Policy (`sdk/`)

> **Audience: contributors** (human or AI) adding or changing a language SDK.

Every SDK is a thin binding over the engine: it calls the shared C library
(`engine/capi`, header `engine/capi/include/xlsx_graft.h`) and never
reimplements what the engine does. A capability that is missing is missing in
the engine and gets added there.

`sdk/ruby` is the first SDK and the reference: where this page leaves a shape
open, a later SDK reads what Ruby did rather than deciding again.

## Supported language versions

The policy, which outlives any version number: **support every upstream
release line that is not end-of-life, except those whose upstream support ends
within six months.** A floor that reaches end-of-life soon after a release
helps nobody. Re-derive the numbers below when adding an SDK and at each
release.

| Language | Floor | Latest stable | Note |
| --- | --- | --- | --- |
| Ruby | 3.4 | 4.0 | 3.3 reaches end-of-life 2027-03 |

**Every supported line is gated, and the required check covers them all.** A
package that only works on the newest interpreter has not supported the
version it claims, and a floor that is only checked on the side stays red
until somebody looks. So `make verify`, and CI's required `make verify` check,
run each SDK on every line in this table, the floor and the latest included.
The price is time: each line is a run of its own. While iterating, `LATEST=1`
narrows a gate to the latest stable line, and `RUBY_VER=<line>` to one line.
CI never sets `LATEST`: it runs one job per line, each with `RUBY_VER=<line>`.

The lines are listed in the language's `mk/sdk/<lang>.mk` (`RUBY_LINES` in
`mk/sdk/ruby.mk`, the floor first and the latest last), the one list the gates
and CI read: CI reads it through `make ci:matrix` and runs one job per line.
Each line is pinned there to a patch release, so a commit gates on the same
interpreter every day. (The base image's Debian layers still move when Docker
Hub rebuilds the tag.)

## The gate, per language

Every SDK has the same four targets, named job first and language last, and
`sdk:verify` / `sdk:test` / `sdk:lint` run the job for every language. A
language's are defined in `mk/sdk/<lang>.mk`, the every-language ones in
`mk/sdk.mk`:

| Target | What it does |
| --- | --- |
| `sdk:verify:<lang>` | the whole bar: lint, the tests at 100% line coverage, then the package built, installed into a directory of its own and loaded from there |
| `sdk:test:<lang>` | the tests at 100% line coverage |
| `sdk:lint:<lang>` | the linter and formatter checks; needs no engine build |
| `sdk:lock:<lang>` | re-resolve the lockfile after a manifest edit, on the floor, so it never picks a dependency the floor cannot install (each later line's gate checks that line). Writes the file and prints its raw output, like `engine:lock` |

The first three are gates like the engine's: one PASS/FAIL line, the full log
in `.make-logs/`, `V=1` for the raw output. They run the supported lines one
after another and stop at the first red one; `LATEST=1` and `<LANG>_VER=<line>`
narrow them.

- **Everything runs in Docker.** Nobody needs the language installed. The
  image is built from `sdk/<lang>/Dockerfile` on every gate, with `sdk/<lang>`
  as the build context, and is never published: it holds the interpreter and
  the locked dependencies only, and Docker's layer cache keeps a rebuild to a
  no-op until the lockfile changes. Its tag carries a hash of the files it is
  built from, so two working trees gating at once never share one image.
- **The lockfile is committed** and the image installs from it in frozen mode,
  as every engine gate is `--locked`. Without it the linter's version would
  move between two runs of the same commit. The lockfile names the platforms of
  CI and of an Apple Silicon Mac's Docker (`x86_64-linux`, `aarch64-linux`).
- **The engine library is never built in a language image.**
  `make engine:capi-lib` builds the release library once, for the Docker host's
  architecture, into the gitignored `dist/capi/local/`. The gate mounts the
  repository at `/repo` and points `XLSX_GRAFT_LIBRARY` at it; it is not copied
  into the image, so changing the engine never rebuilds one.
- **The image's libc must be at least as new as the library's.** The library is
  built on bookworm (glibc 2.36). glibc loads a library built against the same
  or an older glibc; a musl base (alpine) loads nothing.

A language's make file, `mk/sdk/<lang>.mk`, is included by the `Makefile`
after `mk/sdk.mk`. It sets its values under fixed names (`<LANG>_LINES`,
`<LANG>_PATCH_<line>`, the `<LANG>_VER` knob and `<LANG>_IMAGE_INPUTS`;
`mk/sdk.mk`'s header shows Ruby's) and hands them to `sdk_lang`, which defines
from them what every language shares: the lines a gate runs, one make per
line, the CI jobs, the refusal of a line not in the list, the image's tag and
the `docker run` the recipes run in. The file itself writes only what is the
language's own: the image build, the four targets above and their recipes.

A language joins the gates by its name in `SDK_LANGS` (`mk/sdk.mk`), which
makes its private `_sdk-verify-<lang>` a member of `_sdk-verify`, and so of
`_verify` in the `Makefile`: `make verify` runs each `sdk:verify:<lang>` on
every line. CI runs the same thing in parallel: `make ci:matrix` lists one job
per line (`make sdk:verify:ruby RUBY_VER=3.4`, …) from each language's
`CI_JOBS_<lang>`, which `sdk_lang` defines, and refuses a language without
one. The required `make verify` job passes only when every one of them does.
Each SDK keeps its own engine build cache, which its lines share
(`.github/actions/engine-cache`).

## Finding the engine library

An SDK loads the first of:

1. the path in the `XLSX_GRAFT_LIBRARY` environment variable (an empty value
   counts as unset)
2. the path the caller passed
3. the copy shipped inside the package

The environment comes first because where the engine lives is the deployment's
decision, and it has to be able to override application code. **Nothing
downloads anything**, at install time or at run time.

When none is found, or the file does not load, the SDK raises (or returns its
language's error) with the three ways above spelled out. That is a setup
mistake rather than a failed call, so it is not a result.

The library's file name is `libxlsx_graft_capi.so` / `.dylib`, and
`xlsx_graft_capi.dll` on Windows (no `lib` prefix).

## Calling the C ABI

- **Declare every function with its types.** An FFI's default return type is
  usually a C `int`, which cuts a returned pointer in half.
- **Copy out exactly the returned length.** Strings cross as a pointer plus a
  length and are not NUL-terminated (rule 1 of the header). Never read one as a
  C string.
- **A call returns a result, not an exception**: whether it worked, the value
  when it did, and diagnostics either way. A status other than `XLSX_GRAFT_OK`
  becomes a failed result carrying one diagnostic, and the out-parameters are
  not read. The diagnostic codes name the status codes for now (`null_arg`,
  `panic`, `unknown_status`); the engine's own diagnostics replace them.
