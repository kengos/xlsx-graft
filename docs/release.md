# Releasing

How a version of xlsx-graft is published: the order of the steps, what each
registry needs set up once, what to do when a step fails, and what cannot be
undone. Everything that can be checked before a release is checked by `make
verify`; the steps here are the ones that change the outside world, and are done
by a person.

The one-time accounts and settings, and the checks after the first publish, are
in [issue #155](https://github.com/kengos/xlsx-graft/issues/155). Nothing in
this repository publishes by itself: both workflows below start only by hand.

## What is published where

| Registry | What | How |
| --- | --- | --- |
| crates.io | the CLI, MCP server and engine crates (`scripts/release/crates.txt`) | OIDC trusted publishing |
| RubyGems | `xlsx_graft`: a gem with no engine and a platform gem for each platform | OIDC trusted publishing |
| PyPI | `xlsx-graft`: a wheel with no engine and a platform wheel for each platform | OIDC trusted publishing |
| npm | `xlsx-graft` and `@xlsx-graft/<os>-<cpu>[-gnu\|-msvc]` for each platform | OIDC trusted publishing, with provenance |
| NuGet | `XlsxGraft`: one nupkg with the engine of every platform | OIDC trusted publishing |
| Maven Central | `io.github.kengos:xlsx-graft` and a classifier jar for each platform | a Portal token and a GPG key, in secrets |
| Packagist | `kengos/xlsx-graft` | a split repository, `kengos/xlsx-graft-php` (below) |
| Go | `github.com/kengos/xlsx-graft/sdk/go` | the tag `sdk/go/v<version>` |
| GitHub Releases | the CLI archives, which the PHP and Go SDKs run | a draft release for the tag `v<version>` |

The platforms are the rows of `scripts/release/platforms.tsv`. The PHP and Go
packages carry no engine: they run the CLI (docs/sdk.md § Platform packages).

## Before the first release

The owner does these once (issue #155 lists them). None can be done by a
workflow, and none is done by this repository's code.

- Create the `release` environment, with a required reviewer and deployment
  branches and tags limited to `main` and `v*` tags. Every publishing job runs in
  it, so nothing is published before the reviewer approves.
- Register each trusted publisher for the owner `kengos`, the repository
  `xlsx-graft`, the workflow file `publish.yml` and the environment `release`:
  crates.io (per crate), RubyGems, PyPI, npm (per package) and NuGet. Renaming
  `.github/workflows/publish.yml` breaks all five.
- crates.io and npm register a trusted publisher only for a package that already
  exists. So the first version of each crate (twelve) and of each npm package
  (the entry and one for each platform) is published once by hand with a
  personal token, and then registered; later versions go through the workflow.
  PyPI and RubyGems register a pending publisher for a name that does not exist
  yet, and need no first publish by hand. NuGet's trusted publishing is set on
  the account.
- Create the secrets `CENTRAL_API_TOKEN`, `GPG_PRIVATE_KEY` and `GPG_PASSPHRASE`
  as secrets of the `release` environment, not of the repository, so that no
  other workflow can read them (the namespace `io.github.kengos` verified in the Central Portal, and the
  public half of the key on keyservers) and `SPLIT_REPO_TOKEN` (a fine-grained
  token with Contents read and write on `kengos/xlsx-graft-php` and nothing
  else; also a secret of the `release` environment).
- Create `kengos/xlsx-graft-php`, empty, and add it to Packagist. Add a ruleset
  to it that restricts creating and deleting tags to the owner, who is also the
  holder of `SPLIT_REPO_TOKEN`.
- Enable private vulnerability reporting (SECURITY.md points to it).

## Making a release

1. **Raise the version** in one pull request: `[workspace.package] version` and
   the version of every crate dependency in `engine/Cargo.toml`, and the Ruby,
   Python, .NET, Java and JS manifests. `make version:check` lists what is left,
   and `make verify` must pass. Add the CHANGELOG entry, and merge it.
2. **Build the packages** from the merge commit on `main`:

   ```sh
   gh workflow run release-artifacts.yml --ref main
   ```

   It builds the engine on a runner of each platform, assembles the packages and
   uploads them as the artifact `packages`. Note the run's id. Check the macOS
   and Windows jobs: they were not built by hand before the first release.
3. **Dry-run the publish** on `main`, with every registry, before any tag exists:

   ```sh
   gh workflow run publish.yml --ref main \
     -f artifacts_run_id=<run id> -f registries=all
   ```

   `dry_run` is true unless it is set to false. Approve the `release`
   environment. Each job does everything up to the last action before its
   registry is touched and makes no credential: `twine check`, `npm publish
   --dry-run`, the Maven bundle made, signed and checked, the split made, and so
   on. Read the log of each job. A failure here publishes nothing, and no tag has
   been pushed yet.
4. **Tag** the same commit with `v<version>` and `sdk/go/v<version>` and push
   both. They are the owner's to push; the workflow only checks them. Do this
   only after the dry run passes: the Go proxy caches a tag for good once a
   user's tool has asked for it.
5. **Publish** with `--ref v<version>` and `-f dry_run=false`, the same
   `artifacts_run_id`. Start with the GitHub Release, which the PHP and Go SDKs
   need (they run the CLI from it):

   ```sh
   gh workflow run publish.yml --ref v<version> \
     -f artifacts_run_id=<run id> -f registries=release -f dry_run=false
   ```

   Approve the `release` environment, then publish the draft release it made.
6. **Publish the registries**, all at once or a few at a time
   (`-f registries=all`, or `-f registries="crates rubygems pypi"`), with
   `-f dry_run=false`. Approve the `release` environment (the jobs that are
   waiting can be approved together). The registries do not depend on one
   another, so they run in parallel, and a failed one is started again alone
   (below). The order inside a job matters and is the job's: the crates in the
   order of `scripts/release/crates.txt`, the gem with no engine before the
   platform gems, the npm packages of the platforms before the main package
   `xlsx-graft`, which names them. `release` is selected again by `all`, and
   leaves the release that exists alone.
7. **Maven Central**: the job uploads the bundle and waits for the Portal to
   validate it. Open <https://central.sonatype.com/publishing/deployments>,
   check the deployment and click Publish. That cannot be undone.
8. **Check what was published**: install each package from its registry on each
   platform and run it (issue #155 lists the checks). Packagist shows the new
   version once its webhook on `kengos/xlsx-graft-php` has run; update it by hand
   from the package's page if it has not.

## What the workflow checks

`prepare` stops the run before any registry job starts unless:

- the tag is `v<version>`, `<version>` being `[workspace.package] version` (a
  dry run may be started on a branch, where there is no tag to compare);
- the run in `artifacts_run_id` is a successful `Release artifacts` run of this
  very commit, so that a package built from other code is never published;
- every file of the `packages` artifact matches `SHA256SUMS`, every platform's
  package holds what it should (`scripts/release/check_packages.py`) and the
  versions agree (`make version:check`).

Each job that publishes a package downloads the artifact again and checks
`SHA256SUMS` against the checksum `prepare` output, because it runs on a runner
of its own. The crates, Packagist and Go jobs publish the source at the tag and
download nothing.
`make release:dry-run:workflow` fails when the workflow can start any other way
than by hand, when a job outside `prepare` is not in the `release` environment,
when a job names a secret or an OIDC permission it does not need, or when an
action is not pinned to a commit.

## When a step fails

A published version is never changed on any registry, so every job but Maven
Central skips a version that is already there, and a failed run is started
again with only the registries that did not finish (`-f registries="npm
nuget"`). `fail_if_published` turns a skip into a failure, which detects a
version that was believed unpublished and is already published.

| Failed | What to do |
| --- | --- |
| `prepare` | Read the message: a wrong tag, an artifacts run of another commit, a checksum. Fix it and start again; nothing was published. |
| crates.io | Start `crates` again: the crates already on crates.io are skipped, and the rest go in order. |
| RubyGems, PyPI, NuGet | Start the job again. A gem push that timed out may have landed, and is read as published. |
| npm | Start `npm` again: a package already on npm is skipped. The package that names the platform packages goes last, so a failure before it leaves nothing that refers to a missing package. |
| Maven Central | Do not upload the same version twice: the Portal refuses a published version, and a second upload may leave a deployment that fails validation, or a second one waiting for the click. If validation failed, drop the failed deployment in the Portal, fix the cause, and start `maven` again; otherwise leave `maven` out of the registries of later runs. |
| Packagist | Start `packagist` again. The split is the same commits every time; the push is not forced, so a destination that was changed by someone else fails instead of being overwritten. |
| Go | The tag is the release. If the proxy does not have it, start `go` again, or request `https://proxy.golang.org/github.com/kengos/xlsx-graft/sdk/go/@v/v<version>.info`. |
| GitHub Release | Delete the draft and start `release` again. |

A version that reached a registry with a defect is not removed: publish the next
version. crates.io and npm allow yanking and deprecating; RubyGems and PyPI allow
yanking; NuGet allows unlisting. Maven Central and the Go proxy keep every
version.

## What cannot be undone

- A version on crates.io, RubyGems, PyPI, npm, NuGet, Maven Central or the Go
  proxy. The name and the version are never free again.
- The Maven Central click in the Portal.
- A tag that a user's tool has already read (the Go proxy caches it for good).
  Do not move `sdk/go/v<version>` or `v<version>` after the proxy has seen it.

## The split repository

Packagist reads `composer.json` at the root of a repository, and in this one it
is `sdk/php/composer.json`. `scripts/release/split-php.sh` therefore pushes
`sdk/php` (with its own copies of the license files, which
`scripts/check-license-copies.sh` keeps equal to the repository's) as the
history of `kengos/xlsx-graft-php`, and the tag `v<version>` on the same commit.
The split is made only from a tag that exists in this repository and points at
the commit being released, so the split repository never holds a version this
one has not released. `main` is pushed without force, and a tag that already
exists on the destination at another commit fails the run.
