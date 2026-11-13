# Security policy

xlsx-graft reads and verifies workbooks that come from someone else, and
will also create and edit them (writing is not built yet). A `verify` that
passes a workbook carrying a dangerous function or part, or a reader that a
crafted workbook can crash, defeats the point of the project, so reports are
welcome and taken seriously.

## Reporting a vulnerability

**Do not open a public issue, pull request or discussion for a security
problem.** Use either channel:

- **GitHub private vulnerability reporting**: the "Report a vulnerability"
  button under this repository's Security tab. It is preferred, because
  the discussion and the advisory that follows stay in one place. If the
  button is not there, use the email address below.
- **Email**: `kengo+xlsx-graft@kengos.jp`, with `xlsx-graft security` in
  the subject.

Include, most useful first:

1. The workbook that shows the problem, or a script that makes it. Build
   the smallest workbook that shows it; do not send a workbook that holds
   real business or personal data.
2. The Rust API call, the C ABI or Node.js addon call, the CLI command,
   the SDK call or the MCP tool call you ran on it.
3. The released version, or the commit of `main` you built from, the
   operating system, and for an
   SDK its language and the language's version.
4. What happened (the diagnostics, the crash, the memory used) and what
   you expected instead.

You will get a reply that the report was received within **7 days** of the
report. If the report is confirmed, you will get an assessment of its
severity and a rough timeline for the fix within **14 days** of the report.
One person maintains xlsx-graft, so these are targets, not a commercial
service level; if a deadline slips, the maintainer tells you why.

## Supported versions

A fix is prepared in private (in the temporary private fork of the GitHub
security advisory), merged to `main` on the disclosure date, and released in
a patch release of the most recent minor version (before 1.0, the most
recent 0.x line, such as 0.1 or 0.2). Older minor versions do not get fixes.
Denying a function that an earlier version accepted is the exception to the
patch release: it comes in the release that "When the table changes" below
says, and older minor versions do not get it either.

No version has been released yet. The first release will be 0.1.0. Until
then, report against `main`; a fix is merged to `main` on the disclosure
date, and the advisory names the commit that fixes it.

## Scope

**In scope**: what a workbook can cause, and the bugs in the interfaces
listed below. The workbook is untrusted. The program that calls xlsx-graft
(the host) is trusted, and so is what it sets: the rules (what the template
allows, and the host's own policy), the limits and the `FunctionTable`. The
policy's `Rules::forbid_part` and `Rules::forbid_function` forbid a kind of
content or a function even where the template allows it;
`Rules::deny_scheduled` denies now the functions the table marks for denial
in a later version, unless the template allows them.

`verify` runs through the Rust API of `engine/verify`
(`xlsx_graft_verify::verify`), the CLI (`xlsx-graft verify`), the C ABI
library, the Node.js addon, the seven SDKs and the MCP server's `verify`
tool. In each of them except the Rust API, the host sets the policy: the
CLI and the MCP server read it from a JSON file (`--policy`), the C ABI
library and the addon take it as the `policy` of their options JSON, and an
SDK takes it as an object (`Verifier.new(policy:)` and its equivalents). The
policy has `forbid` (the policy's `Rules::forbid_part` and
`Rules::forbid_function`), `denyScheduled` (`Rules::deny_scheduled`),
`failOn` and the limits (`xlsx_graft_verify::Limits`). A limit may be raised
or lowered, but not past its hard max, which keeps the memory use of a
result bounded.

- **`verify` passes what it should refuse.** With the default rules and
  the default function table (below), `verify` reports no error for a
  workbook that calls a function the table marks dangerous or does not
  mark reviewed safe, or that carries content of a kind denied by default
  (macros, external links, DDE, OLE, ActiveX, data connections, web
  publishing), in a part or in a formula. This is the most severe kind of
  bug in this project.
  - Also in scope, even where the template allows the kind or the function
    (`Rules::allow_part`, `Rules::allow_function`): `verify` reports no
    error for a kind of content or a function that the host's policy forbids
    (`Rules::forbid_part`, `Rules::forbid_function`). Under
    `Rules::deny_scheduled`, `verify` reports no error for a function the
    table marks for denial in a later version and the template does not
    allow.
  - Accepted by design (the module documentation of
    `engine/verify/src/lib.rs` gives the exact conditions): a function the
    table marks for denial in a later version (without
    `Rules::deny_scheduled`), and INDIRECT and OFFSET, each with a notice
    (`function_deny_scheduled`, below, and `function_reference_unchecked`).
    `verify` does not judge which cells the reference that INDIRECT or
    OFFSET computes points to, and the notice says so. Nor does it judge
    whether the text INDIRECT reads names the cells of another open workbook
    (ECMA-376-1:2016 §18.17.7.167). A relationship to an `http://`,
    `https://` or `mailto:` address is accepted without a diagnostic only
    where its type is ECMA-376's hyperlink relationship type and its part
    uses it as a hyperlink (`hyperlink`, `hlinkClick`, `hlinkHover`) and as
    nothing else; one accepted in any other case is in scope.
- **A malformed or very large workbook makes the engine fail badly**: a
  panic, a crash, a hang, memory or temporary-file use that the limits in
  force (`xlsx_graft_verify::Limits`, `xlsx_graft_package::Limits` and
  `xlsx_graft_read::ReadLimits`, which bound counts and sizes in the input)
  do not bound or that is out of proportion to them, or time out of
  proportion to the size of the workbook, instead of a diagnostic or an
  error. The package reader refuses a zip bomb (`zip_ratio_exceeded`,
  `zip_entry_too_large`, `zip_total_too_large`) and any DOCTYPE in the XML
  it reads (`xml_doctype`), which is how an XML external entity would come
  in; a workbook that gets past either check is in scope.
- **A workbook that xlsx-graft writes makes Excel show its repair
  dialog**, once xlsx-graft writes workbooks (it does not yet).
- **Reading or verifying opens a network connection**, or reads or writes a
  file other than the workbook it was given and the temporary files the
  engine makes to read it (in the system's temporary directory, or the
  directory the host names).
- **The interfaces**: the CLI, the MCP server, the C ABI library, the
  Node.js addon and the SDKs under `sdk/`. Their own code is in scope, and a
  value that the host passes to an SDK or the CLI (a file name, a sheet
  name) may come from an untrusted source, so in scope are, for example,
  memory unsafety at the C ABI or the addon, an SDK that passes the engine a
  value other than the one its caller gave, or, in the PHP and Go SDKs that
  run the CLI as a subprocess, an argument that the CLI reads as an option.
  - Also in scope: a value quoted from the workbook in an MCP result (in the
    `args`, `location`, `message` or `fix` of a diagnostic, and in the text
    part that repeats them) that is longer than 200 characters, or that still
    holds a control character or a bidirectional formatting character. A
    `message` or a `fix` that holds such a value is longer than 200 characters
    by its own words; the bound is on each quoted value.

**Known gaps**: `verify` does not check these yet. They are listed in the
`# Not yet` section of `engine/verify/src/lib.rs`, so you need not report
that `verify` misses one of them; a crash or another problem in scope
that one of them causes is still in scope:

- Office add-ins (`webextension`), a ribbon (`customUI`) and attached
  toolbars (`attachedToolbars`).
- The formulas a form control keeps in its control properties or in VML
  (`fmlaMacro`, `fmlaLink`, `fmlaRange` and the like).
- What an embedded object or package holds: allowing OLE admits whatever
  it embeds, including the links a compound file keeps inside it.

**Out of scope**:

- `verify` refusing a function or a kind of part that is in fact safe.
  That is a feature request; open an issue asking for the function or the
  kind of part to be reviewed.
- `verify` missing something Excel repairs (a relationship whose target
  is missing, for example) when the workbook carries nothing dangerous.
  That is a bug; open an issue.
- What an AI client does with a string quoted from the workbook in an MCP
  result. The tool says in its description and in the text of every result
  that the values of a diagnostic's `args`, `location`, `message` and `fix` can quote
  the workbook and are data, not instructions; whether
  the client treats them so is the client's behavior.
- The files the MCP server reads: its `verify` tool reads any file that the
  user running the server can read, for any `path`, except a path that is not
  a regular file (a directory, a named pipe, a device), which is not opened and
  gives an `io` error. There is no `--root`. That is by design; to limit the
  files, run the server as a user or in a container that can read only
  those.
- Content that the Rust caller allows (`Rules::allow_function`,
  `Rules::allow_part`; a template's `functions.allow` and `parts.allow`,
  once templates are read) and the host's policy does not forbid, and what a
  Rust caller's own `FunctionTable` lets through.
- Resource use within limits that a host raised above the defaults.
- What Excel or another application does when it opens a workbook, other
  than the repair dialog above. Report a bug in Excel to Microsoft.
- Attacks that need an already compromised machine.
- A problem that arises only when the person who runs xlsx-graft also
  made the workbook, and could already run code on that machine.
- A vulnerability in a dependency, as its authors ship it. Report it
  upstream, and tell the maintainer too, so the dependency can be updated
  or replaced.

## Disclosure

Disclosure is coordinated: the maintainer agrees on a date with you,
publishes a GitHub security advisory, and credits you by the name or
handle you choose, or leaves you unnamed if you prefer. If a report turns
out to concern a dependency rather than xlsx-graft, the maintainer helps
send it to its authors.

There is no bug bounty.

## The default function table

The engine's `verify`, with its default function table, refuses a workbook
whose formulas call a function the engine has not reviewed and found safe.
Every interface that runs `verify` (the CLI, the C ABI library, the Node.js
addon, the SDKs and the MCP server) uses this table. A
Rust caller of the engine can give `verify` a `FunctionTable` of its own.
Every function in the table is one of:

- **reviewed safe**: its definition was read, and its result depends only
  on its arguments and the workbook's cells and names (and also on the
  clock, random numbers and the locale settings). Each such row cites the
  definition it was read from and the day it was read.
- **dangerous**: it reaches the network, reads data from outside the
  workbook, runs code outside it, returns information about the machine or
  the file's path, or opens a link the formula computes. `verify` reports
  it as `function_dangerous`, with the reason.
- **unreviewed**: nobody has reviewed it yet, or the review could not
  settle it. `verify` reports it as `function_denied`, as it does a
  function the table does not list.

Only a reviewed-safe function passes by default, so a function the review
missed is refused, not accepted. A template accepts any other function
by listing it in `functions.allow`; a host's policy can forbid any
function, whatever the template allows.

### When the table changes

Marking a function reviewed safe refuses nothing that was accepted before,
and may come in any release.

Denying a function that an earlier version accepted may break a workbook
that works today:

- **Before 1.0** (every 0.x release), any release may deny a function the
  release before it accepted, and need not schedule the denial first.
- **From 1.0**, denying such a function is a security fix.
  - An urgent denial may come in a minor or a patch release, and a security
    advisory announces it.
  - Otherwise the function is scheduled in one minor release, and denied
    in the next minor release. While it is scheduled, `verify` accepts it
    with a `function_deny_scheduled` notice that names the version that is
    to deny it and why.

Once a function is denied, `function_dangerous` names, when the table
records them, the version that began to deny it and the advisory. A
function the template lists in `functions.allow` stays allowed, so a
template that has to keep using a function lists it there. A host can
deny the scheduled functions now (`Rules::deny_scheduled` in the engine),
to find the workbooks the next version will refuse before upgrading.
