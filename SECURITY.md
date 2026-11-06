# Security policy

## The default function table

The engine's `verify`, with its default function table, refuses a workbook
whose formulas call a function the engine has not reviewed and found safe.
Every interface that runs `verify` (the CLI, the SDKs and the MCP server,
once they run it) uses this table. A Rust caller of the engine can give
`verify` a `FunctionTable` of its own. Every function in the table is one
of:

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
