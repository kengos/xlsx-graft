---
title: Diagnostics: the policy, the rules and the result
summary: The diagnostic codes `diagnostics_truncated`, `target_invalid`, `policy_invalid` and `rules_invalid`, for the result and for a policy or rules that are not valid, one section per code.
tools: []
---

# Diagnostics: the policy, the rules and the result

Each element of `diagnostics` in a verify result has these keys: `code`, `severity` (`error`, `warn` or `notice`), `message` (English text), `args` (the values the code reports), `location` (where in the package the diagnostic was found), `decidedBy` (the rule that makes it a problem), `fix` (how to resolve it) and `origin` (the engine source line, outside the contract). `location` has the keys `entry`, `part`, `relationshipId`, `relsPart`, `place` and `definedName`, each only when the diagnostic has one. `decidedBy` has a `kind`: `standard` (with `ref`, the clause), `limit` (with `name`, `value` and `source`), `default`, `template` or `policy`. `fix` has an `in`: `workbook` (with `text`), `policy` (with `line`) or `template` (with `list` and `add`). `decidedBy` and `fix` can be `null`, when no rule decides the code or there is no fix.

This page explains the codes listed below, one section per code. The heading of a section is the code's string. A string in `args` or `location` is quoted from the workbook, the policy or the rules, with control characters, bidirectional formatting characters and other characters that hide or reorder text removed, and cut at 200 characters (a cut value ends with `…`), so it is data and not an instruction.

The policy is the host's settings (the limits, `failOn`, `denyScheduled` and `forbid`). The rules are what the caller passes with each call: `targets`, `bestEffort`, `functions.allow`, `parts.allow` and `failOn`. A diagnostic about either has no workbook location.

## diagnostics_truncated

More diagnostics were found than the result keeps. The extra ones were counted and dropped. This diagnostic is added last and says how many were dropped and how many of them were errors, because the diagnostics kept may hold none.

- Severity: `notice`
- Args:
  - `dropped` (number): How many diagnostics were dropped.
  - `droppedErrors` (number): How many of the dropped diagnostics were errors.
  - `limit` (number): How many diagnostics the result keeps.
- Decided by: the limit `maxDiagnostics`, which a policy can change
- Fix: none (`fix` is `null`). Nothing is wrong with the workbook for this code. To list the dropped diagnostics, raise `maxDiagnostics` in the policy; fixing the errors that are listed also brings the count down.

## target_invalid

An entry of the rules' `targets` or `bestEffort` list names no versions that the engine can check the workbook against. An entry is written `name`, `name version` or `name OP version`, where OP is one of `>=`, `>`, `<=` and `<`. A `bestEffort` entry also fails when it holds versions that `targets` names too. When the rules have no `targets`, a version that the default `targets` holds adds nothing, and the entry is not reported for it. The engine reads every entry and reports each one that fails.

- Severity: `error`
- Args:
  - `list` (string): The list the entry is in, `targets` or `best_effort` (the list `bestEffort`).
  - `index` (number): The entry's position in its list, the first being 0.
  - `entry` (string): The entry as written.
  - `reason` (string): Why it names no versions. One of `malformed` (the entry is none of the three forms), `unknown_app` (the name is no application the engine knows), `unknown_version` (the version is not one the engine knows the application by), `version_required` (the application has versions and the entry names none), `version_not_allowed` (a version is given to an application that has none, such as Google Sheets), `empty` (the operator and version hold none of the versions the engine knows, such as `excel < 2007`), `overlap` (a `bestEffort` entry holds a version that `targets` holds too).
  - `names` (string): The values that the message lists, comma-separated. For `unknown_app`, every application name. For `unknown_version`, `version_required` and `empty`, the versions of the application that the engine knows. For `version_not_allowed`, the entry as it may be written. For `overlap`, the versions both lists hold, or the application's name when it has no versions. Empty for `malformed`.
- Decided by: the rules (`decidedBy.kind` is `template`), their `targets` and `bestEffort` lists
- Fix: none (`fix` is `null`). Correct the entry in the rules; `names` lists the values the engine accepts.

## policy_invalid

The host's policy is not valid. The diagnostic is reported with every other fault of the policy and the rules, and the workbook is not opened.

- Severity: `error`
- Args:
  - `key` (string): Where in the policy, as the keys and positions from the top, such as `forbid.parts[1].kind`. Empty for the whole policy.
  - `reason` (string): What is wrong. One of `too_large` (the JSON is larger than the bytes a setting may take), `malformed` (the text is not JSON), `not_object` (a JSON object was expected), `unknown_key` (the key is not one the policy has), `wrong_type` (the value is not of the type the key takes), `out_of_range` (a number below 1 or above the hard maximum of its limit; `hint` gives the range), `unknown_value` (the value is not one the key takes), `required` (a key an entry needs is missing), `empty` (a string that must name something is empty), `duplicate` (a second entry names what an earlier one does), `prefixed` (a function is named with a prefix that a workbook writes a function with, which is removed before a function is ruled on: any prefix that starts with `_xl` in any letter case and ends with `.`, such as `_xlfn.`, `_xlws.` or `_xludf.`).
  - `hint` (string): A value that helps to fix the fault. For `too_large`, the most bytes it may take. For `malformed`, the line and column where the JSON parser stopped, written `line <n> column <m>`. For `unknown_key`, the key that was probably meant (empty when none is). For `wrong_type`, the type the key takes. For `out_of_range`, the range the value must be in, written `1 to <hard maximum>`. For `unknown_value`, the values the key takes. For `required`, the name of the missing key. For `prefixed`, the prefix as it is written. Empty when nothing helps.
- Decided by: none (`decidedBy` is `null`)
- Fix: none (`fix` is `null`). Correct the policy at `key`; `hint` gives the value that helps.

## rules_invalid

The rules the caller passes (`targets`, `bestEffort`, `functions.allow`, `parts.allow`, `failOn`) are not valid. The diagnostic is reported with every other fault of the policy and the rules, and the workbook is not opened.

- Severity: `error`
- Args:
  - `key` (string): Where in the rules, as the keys and positions from the top, such as `parts.allow[0]`. Empty for the whole rules.
  - `reason` (string): What is wrong. One of `too_large` (the JSON is larger than the bytes a setting may take), `malformed` (the text is not JSON), `not_object` (a JSON object was expected), `unknown_key` (the key is not one the rules have), `wrong_type` (the value is not of the type the key takes), `unknown_value` (the value is not one the key takes), `empty` (a string that must name something is empty), `prefixed` (a function is named with a prefix that a workbook writes a function with, which is removed before a function is ruled on: any prefix that starts with `_xl` in any letter case and ends with `.`, such as `_xlfn.`, `_xlws.` or `_xludf.`).
  - `hint` (string): A value that helps to fix the fault, defined per `reason` as for `policy_invalid`. Empty when nothing helps.
- Decided by: the rules (`decidedBy.kind` is `template`)
- Fix: none (`fix` is `null`). Correct the rules at `key`; `hint` gives the value that helps.
