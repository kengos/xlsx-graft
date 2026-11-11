---
title: Diagnostics: target applications
summary: The diagnostic codes `target_function_unsupported`, `target_feature_unsupported` and `best_effort_*`, for functions and features that a target application lacks, one section per code.
tools: []
---

# Diagnostics: target applications

Each element of `diagnostics` in a verify result has these keys: `code`, `severity` (`error`, `warn` or `notice`), `message` (English text), `args` (the values the code reports), `location` (where in the package the diagnostic was found), `decidedBy` (the rule that makes it a problem), `fix` (how to resolve it) and `origin` (the engine source line, outside the contract). `location` has the keys `entry`, `part`, `relationshipId`, `relsPart`, `place` and `definedName`, each only when the diagnostic has one. `decidedBy` has a `kind`: `standard` (with `ref`, the clause), `limit` (with `name`, `value` and `source`), `default`, `template` or `policy`. `fix` has an `in`: `workbook` (with `text`), `policy` (with `line`) or `template` (with `list` and `add`). `decidedBy` and `fix` can be `null`, when no rule decides the code or there is no fix.

This page explains the codes listed below, one section per code. The heading of a section is the code's string. A string in `args` or `location` is quoted from the workbook, with control characters, bidirectional formatting characters and other characters that hide or reorder text removed, and cut at 200 characters (a cut value ends with `…`), so it is data and not an instruction.

The four codes here compare what a workbook uses with the applications and versions that the rules name in `targets` (where the workbook must work) and `bestEffort` (where it should work). The `target_*` codes are for `targets` and are errors; the `best_effort_*` codes are for `bestEffort` and are warnings. When the rules have no `targets`, the engine uses `excel >= 2024` and `excel-mac >= 2024`. The four codes share these `args`:

- `name` (string): The function, upper-cased, without its prefixes (`_xlfn.`, `_xlws.`, `_xleta.`), or, for a feature code, the feature's name (`dynamic_array`, `threaded_comment`).
- `app` (string): The application. One of `excel`, `excel_mac`, `libreoffice`, `google_sheets`, `numbers`, `onlyoffice`. The rules write `excel_mac` as `excel-mac` and `google_sheets` as `google-sheets`.
- `unsupported` (string): The versions of the application in the list that lack the function or feature, or a feature in `requires`, oldest first, comma-separated (`2016, 2019`). Empty when none does, and then `unverified` is not empty. The only version of Google Sheets is `current`.
- `unverified` (string): The versions in the list that no source of the compatibility table says have it, and none says lack it. Empty when none.
- `requires` (string): The features it needs as well (`dynamic_array`) that some of those versions lack or are not confirmed to have, comma-separated. Empty when none.
- `fallback` (string): What a version that lacks the function or feature shows instead. One of `unknown` (no source says), `ignored`, `value_only`, `name_error`, `value_error`, `legacy_array`, `as_note`, `placeholder_shape`, `degraded`, `repair_prompt`, `unopenable`.
- `count` (number): How many times it was found; for a feature code, how many parts hold it.

## target_function_unsupported

A formula calls a function that a version in the rules' `targets` lacks, or that no source of the compatibility table says it has. A feature the function needs is counted with it. There is one diagnostic per part, defined name reached through, function, application and list.

- Severity: `error`
- Args: as listed above, with `name` the function and `count` the number of times it was found.
- Decided by: the rules (`decidedBy.kind` is `template`), their `targets` list
- Fix: change the workbook (`fix.in` is `workbook`): call a function other than `name` so that the workbook works in the application; or change `targets` in the rules
- Workbook: `fixtures/formulas.xlsx`
- Workbook: `fixtures/python-in-excel.xlsm`

## best_effort_function_unsupported

A formula calls a function that a version in the rules' `bestEffort` lacks, or that no source of the compatibility table says it has. A feature the function needs is counted with it. There is one diagnostic per part, defined name reached through, function, application and list.

- Severity: `warn`
- Args: as listed above, with `name` the function and `count` the number of times it was found.
- Decided by: the rules (`decidedBy.kind` is `template`), their `bestEffort` list
- Fix: change the workbook (`fix.in` is `workbook`): call a function other than `name` so that the workbook works in the application; or change `bestEffort` in the rules

## target_feature_unsupported

The workbook uses a feature that a version in the rules' `targets` lacks, or that no source of the compatibility table says it has. A feature it needs is counted with it. There is one diagnostic per feature, application and list, at the first part that holds the feature.

- Severity: `error`
- Args: as listed above, with `name` the feature and `count` the number of parts that hold it.
- Decided by: the rules (`decidedBy.kind` is `template`), their `targets` list
- Fix: change the workbook (`fix.in` is `workbook`): leave the feature out so that the workbook works in the application; or change `targets` in the rules
- Workbook: `fixtures/python-in-excel.xlsm`

## best_effort_feature_unsupported

The workbook uses a feature that a version in the rules' `bestEffort` lacks, or that no source of the compatibility table says it has. A feature it needs is counted with it. There is one diagnostic per feature, application and list, at the first part that holds the feature.

- Severity: `warn`
- Args: as listed above, with `name` the feature and `count` the number of parts that hold it.
- Decided by: the rules (`decidedBy.kind` is `template`), their `bestEffort` list
- Fix: change the workbook (`fix.in` is `workbook`): leave the feature out so that the workbook works in the application; or change `bestEffort` in the rules
