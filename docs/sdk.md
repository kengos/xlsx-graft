# SDK Policy (`sdk/`)

> **Audience: contributors** (human or AI) adding or changing a language SDK.

Every SDK is a thin binding over the engine: it calls the shared C library
(`engine/capi`, header `engine/capi/include/xlsx_graft.h`) and never
reimplements what the engine does. A capability that is missing is missing in
the engine and gets added there. A language with no FFI it can count on runs
the engine another way: JavaScript loads a Node.js addon, and PHP and Go run
the engine's CLI (`engine/cli`, binary `xlsx-graft`) as a subprocess
([Running the CLI](#running-the-cli)).

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
| Python | 3.11 | 3.15 | 3.10 reached end-of-life 2026-10-01 |
| .NET | 10.0 | 10.0 | 8.0 and 9.0 reach end of support 2026-11-10; 11.0 is added once it ships |
| Java | 8 | 25 | LTS lines only; 27 (non-LTS) ends 2027-03. Temurin supports 11 and 17 to at least 2027-10: unless that moves, they leave from 2027-04 while 8 stays, so Java's lines need not be consecutive |
| JavaScript (Node.js) | 24 | 26 | 22 reaches end-of-life 2027-04-30, within six months from 2026-10-30, as this SDK lands; 27 is added once it is Current (2027-04-22) |
| PHP | 8.3 | 8.5 | 8.2 reaches end-of-life 2026-12-31, within six months; 8.6 is added once it ships (2026-11-19). 8.3 leaves from 2027-07, six months before its end-of-life (2027-12-31) |
| Go | 1.26 | 1.27 | Go supports its two latest lines, so the older is always within six months of its end: both are supported. 1.28 (about 2027-02) is added, and 1.26 leaves, when it ships |

**Every supported line is gated, and the required check covers them all.** A
package that only works on the newest interpreter has not supported the
version it claims, and a floor that is only checked on the side stays red
until somebody looks. So `make verify`, and CI's required `make verify` check,
run each SDK on every supported line: the floor and the latest in this table,
and every line between them (Python's 3.12 to 3.14, Java's 11, 17 and 21,
PHP's 8.4).
A non-LTS Java line is supported for six months, so it is always within six
months of its end, and the policy leaves Java with its LTS lines alone.
Node.js ships one line a year from 27 on, every one of them becoming LTS, so
its odd lines end with 25 and each new line is added once it is Current. Go
supports its two latest lines, and a line ships every six months, so the
older is always within six months of its end: the policy would always leave
Go with its newest line alone, and both of Go's lines are supported instead.
The price is time: each line is a run of its own. While iterating,
`LATEST=1` narrows a gate to the latest stable line, and `<LANG>_VER=<line>`
(`RUBY_VER`, `PYTHON_VER`, `DOTNET_VER`, `JVM_VER`, `JS_VER`, `PHP_VER`,
`GO_VER`) to one line.
CI never sets `LATEST`: it runs one job per line, each with
`<LANG>_VER=<line>`.

The lines are listed in the language's `mk/sdk/<lang>.mk` (`<LANG>_LINES`:
`RUBY_LINES` in `mk/sdk/ruby.mk`, `PYTHON_LINES` in `mk/sdk/python.mk`,
`DOTNET_LINES` in `mk/sdk/dotnet.mk`, `JVM_LINES` in `mk/sdk/jvm.mk`,
`JS_LINES` in `mk/sdk/js.mk`, `PHP_LINES` in `mk/sdk/php.mk`, `GO_LINES` in
`mk/sdk/go.mk`, the floor first and the latest last), the one
list the gates and CI read: CI reads it through `make ci:matrix` and runs one
job per line.
Each line is pinned there to a patch release, so a commit gates on the same
interpreter every day. The pin (`<LANG>_PATCH_<line>`) is the release of the
tool that decides what the line's gate checks: the interpreter for Ruby and
Python, the SDK for .NET (an SDK release, not a runtime one), whose release
names the image and carries `dotnet format` and the analyzers, and for Java
the Temurin release of the JDK the line's tests run on (as Docker Hub tags it,
`_` where the version has `+`), for JavaScript the Node.js release, for PHP
the PHP release, and for Go the Go release.
Java's tools (Gradle, javac, javadoc, the formatter) run on one JDK for every
line, pinned in `sdk/jvm/Dockerfile` with Gradle, and compile for the floor
(`--release 8`). (The base image's OS
layers still move when the registry rebuilds the tag.)

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
- **The lockfile is committed** and the image installs from it without
  re-resolving it, as every engine gate is `--locked`. Without it the
  linter's version would move between two runs of the same commit. The
  lockfile covers the platforms of CI and of an Apple Silicon Mac's Docker
  (x86-64 and arm64 Linux): Ruby's `Gemfile.lock` names them
  (`sdk:lock:ruby` adds them), and Python's `uv.lock` is universal, every
  platform in one file, as is a NuGet `packages.lock.json` that names no
  runtime identifier (.NET's, one per project), and Gradle's lockfiles,
  since JVM dependencies do not vary by platform (Java's `gradle.lockfile`,
  and `buildscript-gradle.lockfile` for the plugins), and JavaScript's
  `pnpm-lock.yaml`, which carries every platform's optional binaries. The
  formatter Spotless fetches is no configuration Gradle can lock, so its
  version is written in `build.gradle.kts`, and the gate runs `--offline` on
  what the image resolved. PHP's `composer.lock` holds no platform-specific
  package, and is resolved for the floor whichever PHP runs the resolve
  (`config.platform.php` in `composer.json`). Go's SDK requires nothing, and
  its tools are a module of their own, `sdk/go/tools`, pinned by the `tool`
  directive of its `go.mod` and locked by its `go.sum`, which holds every
  platform's hashes: kept out of the SDK's `go.mod`, they never reach a
  user's module graph.
- **The engine library is never built in a language image.**
  `make engine:capi-lib` builds the release library once, for the Docker host's
  architecture, into the gitignored `dist/capi/local/`. The gate mounts the
  repository at `/repo` and points `XLSX_GRAFT_LIBRARY` at it; it is not copied
  into the image, so changing the engine never rebuilds one. JavaScript's
  engine is the Node.js addon instead (below): `make engine:napi` builds it
  the same way into `dist/napi/local/xlsx_graft.node`, and the gate points
  `XLSX_GRAFT_ADDON` at it. The engine of PHP and Go is the CLI (below):
  `make engine:cli-bin` builds it the same way into `dist/cli/local/xlsx-graft`,
  and the gate points `XLSX_GRAFT_CLI` at it.
- **The image's libc must be at least as new as the library's.** The library is
  built on bookworm (glibc 2.36). glibc loads a library built against the same
  or an older glibc; a musl base (alpine) loads nothing. So the base image need
  not be Debian: Microsoft ships no Debian image of .NET 10, and .NET's is
  Ubuntu noble (glibc 2.39), as Java's is (Temurin's noble images; its alpine
  ones are musl). The Node.js addon and the CLI are built on bookworm as well
  (a binary linked against glibc runs on the same or a newer one, as a
  library loads), and JavaScript's image is node's bookworm one, PHP's is
  php's and Go's is golang's.

A language's make file, `mk/sdk/<lang>.mk`, is included by the `Makefile`
after `mk/sdk.mk`. It sets its values under fixed names (`<LANG>_LINES`,
`<LANG>_PATCH_<line>`, the `<LANG>_VER` knob and `<LANG>_IMAGE_INPUTS`;
`mk/sdk.mk`'s header shows Ruby's) and hands them to `sdk_lang`, which defines
from them what every language shares: the lines a gate runs, one make per
line, the CI jobs, the refusal of a line not in the list, the image's tag and
the `docker run` the recipes run in. That `docker run` points
`XLSX_GRAFT_LIBRARY` at the C ABI's library; a language that reaches the
engine otherwise (the Node.js addon, the CLI) sets `<LANG>_ENGINE`, the
`NAME=path` passed to `docker run -e` instead (one without `=`, or one from
the command line or an `override`, stops make; one from the environment is
not read). The file itself writes only what is the language's own: the image
build, the four targets above and their recipes.

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

When none is found, or the file does not load, the SDK raises (as
[Running the CLI](#running-the-cli) defines it) with the three ways above
spelled out. That is a setup mistake rather than a failed call.

The library's file name is `libxlsx_graft_capi.so` / `.dylib`, and
`xlsx_graft_capi.dll` on Windows (no `lib` prefix).

JavaScript is the exception. Node.js has no FFI on the lines it supports, so
the JS SDK loads a Node.js addon, `xlsx_graft.node` (`engine/napi`), which
links the C ABI in statically: one file carrying the whole engine, never the
library above. It looks the addon up in the same order, from
`XLSX_GRAFT_ADDON`, so one environment can point the other SDKs at the
library and JavaScript at the addon.

An SDK that runs the CLI looks it up in the same order too, from its own
variable, so the library, the addon and the CLI can each be pointed at
apart:

1. the path in `XLSX_GRAFT_CLI` (an empty value counts as unset)
2. the path the caller passed
3. `xlsx-graft` on `PATH` (`xlsx-graft.exe` too, on every system, for
   Windows), searched by the SDK itself, so a CLI that is not there is
   reported as missing rather than as a failed run. It stands where the
   packaged copy stands above, until a package ships the CLI. Go skips a
   `PATH` entry that is not absolute, such as `.`, as its `os/exec` refuses
   to run what is found there, and on Windows takes the `.exe` alone.

When a path is set but cannot be run, the SDK raises, as it does for a
library that does not load, rather than falling through to the next way. It
raises too when none is found. A name with no directory in it, set in 1
or 2, is looked up on `PATH` by the system rather than by the SDK.

## Calling the C ABI

In JavaScript the addon's Rust calls the C ABI (`engine/napi/src/call.rs`) and
hands over the status and, on `XLSX_GRAFT_OK`, an owned copy of the value; the
JS SDK maps the status to a result as below.

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

### Reading rows through the C ABI

The header gives the whole contract; this is the shape an SDK builds on.

- **A book, then rows.** `xlsx_graft_book_open_path` / `_open_bytes` make a
  book handle, `xlsx_graft_rows_open` (by index) /
  `xlsx_graft_rows_open_name` (by name, letter case aside) a rows handle
  for one sheet, and `xlsx_graft_rows_next` reads a row a call until it
  writes row number 0 (an empty row has its number and no values). Each
  handle has its close, which an SDK calls however its caller's block
  ends. Closing the book closes the rows of every rows handle opened
  from it, and the file: such a handle answers
  `XLSX_GRAFT_ERR_CLOSED`, and only its own close, which frees what is
  left of it, is still to call. So an SDK need not track its rows
  handles to close the file; it closes the book. The other way round, an
  SDK's rows object holds its book object, so that a garbage collector
  does not finalize the book, and with it the rows, while they are read. The diagnostics are the
  book's, so read them before closing it.
- **Null a handle as you close it.** Using a closed handle is undefined
  behaviour, but a null one is refused with `XLSX_GRAFT_ERR_NULL_ARG` and
  closing it does nothing.
- **One thread at a time** for a book and its rows together, closes
  included: they share state the library does not lock. A finalizer that
  runs on a thread of its own closes only under the program's lock for
  that book, or not at all. After `XLSX_GRAFT_ERR_PANIC` the book and
  its rows answer every call but their closes with it, touching nothing:
  close them, and read no diagnostics. A close that returns it has
  closed its handle all the same.
- **An empty input may be null.** A null pointer with a length of 0 is
  read as no bytes, so an empty array an FFI passes as null needs no
  special case. A path that is not UTF-8 is refused; read that file and
  open its bytes.
- **A cell** is an `XlsxGraftValue`: its `kind` is one of the nine types
  of `read`'s JSON Lines ([below](#reading-rows-read)), in the same order,
  and its fields carry the same value (an elapsed time in milliseconds, a
  date or time in parts). Copy what you keep before the next call: the
  values, and the text they point at, live until then.
- **What is a result.** Opening returns `XLSX_GRAFT_OK` for any file the
  engine could read, a workbook or not; the book's diagnostics
  (`xlsx_graft_book_diagnostics`, read after the rows) say what it found:
  `{"has_errors": B, "error": E, "diagnostics": [...]}`, as `read`'s end
  line says them. `error` is the book's first failure, `{"code", "message"}`
  in `read`'s words, or null: a sheet refused (an unreadable workbook
  from the open on), or `io` when the host failed while reading rows. A
  refused sheet stays the error when another is read after, so an SDK
  looks a sheet up in the list rather than by trying to open it. A read that a problem in the sheet stopped has no `error`, and an
  error diagnostic. A status refusing a sheet maps to the code `read`
  puts in its end line's `error`, the one the JSON then holds:

  | Status | `error.code` |
  | --- | --- |
  | `XLSX_GRAFT_ERR_WORKBOOK_UNREADABLE` | `workbook_unreadable` |
  | `XLSX_GRAFT_ERR_SHEET_NOT_FOUND` | `sheet_not_found` |
  | `XLSX_GRAFT_ERR_SHEET_AMBIGUOUS` | `sheet_ambiguous` |
  | `XLSX_GRAFT_ERR_SHEET_NOT_WORKSHEET` | `sheet_not_worksheet` |
  | `XLSX_GRAFT_ERR_SHEET_UNREADABLE` | `sheet_unreadable` |

  `XLSX_GRAFT_ERR_IO` is the host failing (the file would not open or
  read, a temporary file would not write), as `read`'s exit 1 without an
  end line is; `xlsx_graft_error_message` says why, on the thread that
  made the call.

## Running the CLI

An SDK that runs the engine's CLI (PHP, Go) treats a run the way the others
treat a call into the C ABI: it returns a result, and *raises* only for a
setup mistake. Here and in
[Finding the engine library](#finding-the-engine-library), *raises* means the
language's own way of reporting one: an exception where the language has them
(Ruby, Python, PHP, …), an `error` returned beside the result where it does
not (Go). It is never a result.

- **No shell.** The CLI is started from an array of arguments (`proc_open`
  with an array in PHP, `exec.Command` in Go), and never through a shell.
  On POSIX each argument reaches the CLI as it is, and is never a command.
  Windows has no argument array: the runtime quotes them into one command
  line by the C runtime's rules, and the CLI's runtime splits it again, so
  the CLI is an `.exe`; a `.bat` or `.cmd` would hand that line to `cmd.exe`,
  which reads it by rules of its own. (Go's `os/exec` would try `PATHEXT`'s
  extensions, `.cmd` among them, for a name with none, so on Windows the Go
  SDK adds `.exe` to a set path or name that has no extension.)
- **stdin reads its end at once** (closed, or the null device), so a CLI
  that ever asked for input does not wait. stdout and stderr are read apart:
  stdout is what the call returns, stderr only ever explains a failure.
  Neither is left to fill a pipe while the other is read (PHP sends stderr to
  a temporary file; Go's `os/exec` reads both at once).
- **A run asks for JSON** (`--format json`) and reads exactly one JSON object
  from stdout. `--version --format json` prints `{"version":"0.0.0"}`.
- **What is a result.** A run that started and ended:

  | The run | The result |
  | --- | --- |
  | exit 0, stdout a JSON object with what the call reads (`version` a string) | successful, with the value |
  | exit other than 0, or killed by a signal (but `read`'s exit 1 with its end line, [below](#reading-rows-read)) | failed, diagnostic `exit_status`: the status or the signal, and the start of stderr |
  | exit 0, stdout not JSON, not an object, or without what the call reads | failed, diagnostic `invalid_output`: the start of stdout, if any |

  A diagnostic's message quotes at most 200 characters of the output, cut
  between characters (and marked `...`) and on one line. The codes are
  provisional, as the status codes are: the engine's own diagnostics replace
  them once the CLI reports some.
- **What raises.** A run that never started:

  | The cause | The message |
  | --- | --- |
  | no CLI found, or the one found cannot be started | why, then the three ways of [Finding the engine library](#finding-the-engine-library) |
  | the language cannot start a process at all (PHP: `proc_open` disabled, or no temporary file for stderr) | what is missing, and not the three ways: another CLI would not start either |

- **A signal is not a status.** A CLI killed by one is reported by the
  signal's number, never as if it had exited with it (PHP's `proc_close`
  returns the number as a status, so the SDK reads `proc_get_status`; Go's
  `ExitCode` is -1 for both a signal and a run not ended, so the SDK reads
  the wait status). An end the SDK cannot read at all (Go's, when the
  application ignores SIGCHLD and the system reaps the CLI first) is an
  `exit_status` too, saying why.
- **No timeout, no cancelling and no cap on the output yet.** `--version`
  returns at once and prints one line; they come with the first call that
  can take long or print much.

### Reading rows (`read`)

`xlsx-graft read <path> [--sheet <name> | --sheet-index <n>] --format json`
prints JSON Lines: a line per row as the sheet is read, then one end line.
The CLI's `read --help` gives the whole contract; an SDK that reads rows
reads it a line at a time, so it never holds the sheet.

- **A row** is `{"row": 3, "cells": [CELL, ...]}`: its number, from 1, and
  the value of each column from A to its last cell. Only the rows the sheet
  writes come. A `CELL` is `{"type": TYPE, "value": VALUE}`:

  | `type` | `value` |
  | --- | --- |
  | `empty` | `null` |
  | `string` | a string |
  | `number` | a JSON number with a fraction or an exponent (`3.0`, `1e+16`): read it as a float |
  | `boolean` | `true` or `false` |
  | `date` | `"YYYY-MM-DD"` |
  | `time` | `"hh:mm:ss.sss"` |
  | `datetime` | `"YYYY-MM-DDThh:mm:ss.sss"` |
  | `elapsed` | an integer: milliseconds, not wrapped at a day |
  | `error` | the error value as written (`"#N/A"`) |

  A date, time or datetime is the workbook's wall-clock time with no time
  zone, counted in the workbook's date system and rounded to the
  millisecond; giving it a time zone is the SDK's to decide.
- **The end** is the last line, the one without `row`:
  `{"ok": BOOL, "sheet": {"index": N, "name": NAME} | null, "rows": N,
  "error": {"code": CODE, "message": TEXT} | null, "diagnostics": [...]}`.
  `ok` is false when `error` is set (`workbook_unreadable`,
  `sheet_not_found`, `sheet_ambiguous`, `sheet_not_worksheet`,
  `sheet_unreadable`) or a diagnostic is an error. `sheet` is null only
  when there is no sheet to choose (`workbook_unreadable`,
  `sheet_not_found`, `sheet_ambiguous`). The diagnostics are the engine's
  own, as every host prints them.
- **What is a result**, decided in this order:
  0. the SDK closed stdout itself, its caller having stopped taking rows:
     not a failure, whatever the status and stdout's tail (below);
  1. exit 0 or 1 with the end line last: a result (successful when `ok`),
     with the rows that came;
  2. else an exit other than 0, or a signal: `exit_status`, whatever
     stdout holds. Exit 1 with no end line is the host failing (the file
     would not open or read, a temporary file or the output would not
     write), and stderr says why; the last line may be cut off;
  3. exit 0 with a line that is not JSON, or without the end line:
     `invalid_output`.
- **Stopping early.** An SDK whose caller stops taking rows closes the
  CLI's stdout. A CLI still writing stops reading at the next output it
  flushes and exits 0 without a word or the end line; one that had
  written everything already exits as `ok` says. Either way the SDK does
  not read the status as a failure (rule 0).
