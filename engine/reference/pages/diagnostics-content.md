---
title: Diagnostics: denied parts, formulas and functions
summary: The diagnostic codes `part_denied`, `part_forbidden`, `formula_*`, `defined_names_too_many` and `function_*`, for denied parts, formulas and functions, one section per code.
tools: []
---

# Diagnostics: denied parts, formulas and functions

Each element of `diagnostics` in a verify result has these keys: `code`, `severity` (`error`, `warn` or `notice`), `message` (English text), `args` (the values the code reports), `location` (where in the package the diagnostic was found), `decidedBy` (the rule that makes it a problem), `fix` (how to resolve it) and `origin` (the engine source line, outside the contract). `location` has the keys `entry`, `part`, `relationshipId`, `relsPart`, `place` and `definedName`, each only when the diagnostic has one. `decidedBy` has a `kind`: `standard` (with `ref`, the clause), `limit` (with `name`, `value` and `source`), `default`, `template` or `policy`. `fix` has an `in`: `workbook` (with `text`), `policy` (with `line`) or `template` (with `list` and `add`). `decidedBy` and `fix` can be `null`, when no rule decides the code or there is no fix.

This page explains the codes listed below, one section per code. The heading of a section is the code's string. A string in `args` or `location` is quoted from the workbook, with control characters, bidirectional formatting characters and other characters that hide or reorder text removed, and cut at 200 characters (a cut value ends with `…`), so it is data and not an instruction.

## part_denied

A part holds content of a kind the engine denies by default, and the rules do not allow it. The content is the part itself (a VBA project, an external link, an OLE object) or, at a relationship, a link outside the package. There is one diagnostic per part and kind, however many times the part shows it.

- Severity: `error`
- Args:
  - `kind` (string): The kind of content. One of `macro` (VBA or XLM macros: a VBA project, a macro-enabled workbook, a macro sheet or a dialog sheet), `external_link` (a link to another workbook or to a resource outside the package), `dde` (a DDE link: a value another program serves), `ole` (an embedded or linked OLE object), `activex` (an ActiveX control), `data_connection` (a data connection: a connection, a query table, an XML map's data binding or a Power Query definition), `web_publish` (a Web publishing item, which republishes part of the workbook to a location of its own).
- Decided by: the engine's own rules (`decidedBy.kind` is `default`): this kind of content is denied unless the rules allow it
- Fix: add the kind to the rules' `parts.allow` (`fix.in` is `template`; `fix.list` is `parts.allow` and `fix.add` is the kind), or change the workbook to remove the content
- Workbook: `fixtures/macro.xlsm`
- Workbook: `fixtures/external-link.xlsx`
- Workbook: `fixtures/python-in-excel.xlsm`

## part_forbidden

A part holds content of a kind the host's policy forbids, whether or not the rules allow it. There is one diagnostic per part and kind, as for `part_denied`.

- Severity: `error`
- Args:
  - `kind` (string): The kind of content. One of `macro` (VBA or XLM macros: a VBA project, a macro-enabled workbook, a macro sheet or a dialog sheet), `external_link` (a link to another workbook or to a resource outside the package), `dde` (a DDE link: a value another program serves), `ole` (an embedded or linked OLE object), `activex` (an ActiveX control), `data_connection` (a data connection: a connection, a query table, an XML map's data binding or a Power Query definition), `web_publish` (a Web publishing item, which republishes part of the workbook to a location of its own).
  - `reason` (string): Why the policy forbids it, in the policy's own words.
- Decided by: the host's policy (`decidedBy.kind` is `policy`), its `forbid.parts`; the rules cannot allow it
- Fix: none (`fix` is `null`). The rules cannot admit what the policy forbids, so the only ways out are to remove the content from the workbook, or for the host to change its policy

## formula_malformed

A formula could not be read, so it is refused whole: past the point where the formula reader stopped, what the formula states would be a guess. This code and the other `formula_*` and `function_*` codes below give one diagnostic per part, per defined name the formula was reached through (or none) and per finding. It is located at the first place found (`part`, `place`, and `definedName` when the formula was reached through a name), and `count` counts every time the part's formulas show it or, when the finding was reached through a defined name, every time that name's formula shows it.

- Severity: `error`
- Args:
  - `reason` (string): Where the formula reader stopped trusting the formula, first. One of `undecodable_escape` (an `_xHHHH_` escape that names a surrogate), `upper_case_escape` (an escape written `_XHHHH_`), `unclosed_string` (a string with no closing `"`), `unclosed_sheet_name` (a quoted sheet name with no closing `'`), `unclosed_bracket` (brackets with no closing `]`), `brackets_too_deep` (brackets nested deeper than the formula reader follows), `unknown_character` (a character no part of a formula starts with), `stray_sheet_name` (a quoted sheet name with no `!` or `|` after it), `markup` (an element inside the formula's element).
  - `count` (number): How many times it was found.
- Decided by: the clause or rule depends on `reason`. For `undecodable_escape`, the standard `ECMA-376-1:2016 §22.9.2.19`. For `upper_case_escape`, `brackets_too_deep` and `markup`, `default` (the engine's own rule). For every other reason, the standard `ECMA-376-1:2016 §18.17.2`.
- Fix: change the workbook (`fix.in` is `workbook`): for `brackets_too_deep`, nest the formula's brackets less deeply; for every other reason, save the workbook again from Excel

## formula_too_long

A formula is longer than the limit, so its text was not kept and it is refused unread.

- Severity: `error`
- Args:
  - `size` (number): The size in bytes, as UTF-8, of the longest formula found.
  - `limit` (number): The most bytes of one formula that are kept.
  - `count` (number): How many times it was found.
- Decided by: the limit `maxFormulaBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxFormulaBytes: <size>`

## defined_names_too_many

The workbook part defines more names than the limit. A name past the limit is not one the formulas are judged by, so the workbook is refused.

- Severity: `error`
- Args:
  - `count` (number): How many names the workbook part defines.
  - `limit` (number): The most defined names that are read.
- Decided by: the limit `maxDefinedNames`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxDefinedNames: <count>`

## formula_denied

A formula holds content of a kind the engine denies by default, and the rules do not allow it: a DDE link (`dde`) or a reference to another workbook (`external_link`).

- Severity: `error`
- Args:
  - `kind` (string): The kind of content. Its type is the part kind, which takes the seven values `macro`, `external_link`, `dde`, `ole`, `activex`, `data_connection` and `web_publish`; a formula holds only `dde` (a DDE link: a value another program serves) or `external_link` (a reference to another workbook).
  - `count` (number): How many times it was found.
- Decided by: the engine's own rules (`decidedBy.kind` is `default`): this kind of content is denied unless the rules allow it
- Fix: add the kind to the rules' `parts.allow` (`fix.in` is `template`; `fix.list` is `parts.allow` and `fix.add` is the kind), or change the formula to remove the content
- Workbook: `fixtures/external-link.xlsx`

## formula_forbidden

A formula holds content of a kind the host's policy forbids, whether or not the rules allow it.

- Severity: `error`
- Args:
  - `kind` (string): The kind of content. Its type is the part kind, which takes the seven values `macro`, `external_link`, `dde`, `ole`, `activex`, `data_connection` and `web_publish`; the policy can forbid any of them, and a formula holds only `dde` or `external_link`.
  - `reason` (string): Why the policy forbids it, in the policy's own words.
  - `count` (number): How many times it was found.
- Decided by: the host's policy (`decidedBy.kind` is `policy`), its `forbid.parts`; the rules cannot allow it
- Fix: none (`fix` is `null`). The rules cannot admit what the policy forbids, so the content must be removed from the formula, or the host must change its policy.

## function_denied

A formula calls, or may call, a function that the engine's table does not mark as reviewed safe and has not found dangerous, and the rules do not allow it.

- Severity: `error`
- Args:
  - `name` (string): The function, upper-cased, with `_XLUDF.` before a user-defined one.
  - `denial` (string): Why the engine denies it. `unreviewed` (the engine knows the function but has not reviewed it yet) or `unknown` (the engine does not know it: a user-defined function, one newer than the engine, or a name that is no function).
  - `count` (number): How many times it was found.
- Decided by: the engine's own rules (`decidedBy.kind` is `default`): a function not marked reviewed safe is denied unless the rules allow it
- Fix: add the function's name to the rules' `functions.allow` (`fix.in` is `template`; `fix.list` is `functions.allow` and `fix.add` is `name`), or change the workbook to call another function

## function_dangerous

A formula calls, or may call, a function that the engine's table finds dangerous, and the rules do not allow it. The code is also used for a function the table schedules a later xlsx-graft version to deny, when the host's policy denies scheduled functions now (`denyScheduled`); its `reason` then adds the version and says that the policy denies the function now.

- Severity: `error`
- Args:
  - `name` (string): The function, upper-cased.
  - `reason` (string): Why the engine finds it dangerous.
  - `count` (number): How many times it was found.
- Decided by: the engine's own rules (`decidedBy.kind` is `default`): a dangerous function is denied unless the rules allow it
- Fix: add the function's name to the rules' `functions.allow` (`fix.in` is `template`; `fix.list` is `functions.allow` and `fix.add` is `name`), or change the workbook to call another function
- Workbook: `fixtures/formulas.xlsx`
- Workbook: `fixtures/python-in-excel.xlsm`

## function_forbidden

A formula calls, or may call, a function that the host's policy forbids, whether or not the rules allow it.

- Severity: `error`
- Args:
  - `name` (string): The function, upper-cased.
  - `reason` (string): Why the policy forbids it, in the policy's own words.
  - `count` (number): How many times it was found.
- Decided by: the host's policy (`decidedBy.kind` is `policy`), its `forbid.functions`; the rules cannot allow it
- Fix: none (`fix` is `null`). The rules cannot admit what the policy forbids, so the workbook must call another function, or the host must change its policy.

## function_deny_scheduled

A formula calls, or may call, a function that the engine's table marks as reviewed safe but schedules to deny from a later xlsx-graft version, and the rules do not allow it. The policy neither forbids the function nor denies scheduled functions now, so the function is accepted until that version.

- Severity: `notice`
- Args:
  - `name` (string): The function, upper-cased.
  - `from` (string): The xlsx-graft version that is to deny the function, as `MAJOR.MINOR.PATCH`.
  - `reason` (string): Why that version is to deny it.
  - `count` (number): How many times it was found.
- Decided by: the engine's own rules (`decidedBy.kind` is `default`)
- Fix: to keep using the function after that version, add its name to the rules' `functions.allow` (`fix.in` is `template`; `fix.list` is `functions.allow` and `fix.add` is `name`)

## function_reference_unchecked

A formula calls, or may call, a function whose result is a reference computed as the workbook is calculated (`INDIRECT`, `OFFSET`), and the function is accepted. Verify does not see which cells that reference points to. The text `INDIRECT` reads may name a cell in another workbook. Verify refuses a reference to another workbook only when the formula writes it out and `parts.allow` in the rules does not admit it.

- Severity: `notice`
- Args:
  - `name` (string): The function, upper-cased.
  - `count` (number): How many times it was found.
- Decided by: the engine's own rules (`decidedBy.kind` is `default`)
- Fix: none (`fix` is `null`). The function is accepted, and nothing in the workbook or the call changes what verify can see of the reference it computes.
