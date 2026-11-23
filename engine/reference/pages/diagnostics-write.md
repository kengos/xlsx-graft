---
title: Diagnostics: writing a package
summary: The diagnostic code `write_entry_unreadable`, for a workbook that is edited and cannot be written because a zip entry of it cannot be read, one section per code.
tools: []
---

# Diagnostics: writing a package

Each element of `diagnostics` in a verify result has these keys: `code`, `severity` (`error`, `warn` or `notice`), `message` (English text), `args` (the values the code reports), `location` (where in the package the diagnostic was found), `decidedBy` (the rule that makes it a problem), `fix` (how to resolve it) and `origin` (the engine source line, outside the contract). `location` has the keys `entry`, `part`, `relationshipId`, `relsPart`, `place` and `definedName`, each only when the diagnostic has one. `decidedBy` has a `kind`: `standard` (with `ref`, the clause), `limit` (with `name`, `value` and `source`), `default`, `template` or `policy`. `fix` has an `in`: `workbook` (with `text`), `policy` (with `line`) or `template` (with `list` and `add`). `decidedBy` and `fix` can be `null`, when no rule decides the code or there is no fix.

This page explains the codes listed below, one section per code. The heading of a section is the code's string. A string in `args` or `location` is quoted from the workbook, with control characters, bidirectional formatting characters and other characters that hide or reorder text removed, and cut at 200 characters (a cut value ends with `…`), so it is data and not an instruction.

## write_entry_unreadable

A zip entry of the workbook being edited cannot be read, so the edited package is not written. Copying the entry would carry its problem into the output. The diagnostic is reported once for each such entry, and its `location` is the entry. It comes after the diagnostic that says why the entry cannot be read when the same diagnostics are collected while the package is read and while it is written; when the host collects them separately, that diagnostic is in the one for reading the package.

- Severity: `error`
- Args: none
- Decided by: none (`decidedBy` is `null`). The other diagnostic of the entry has the rule.
- Fix: change the workbook (`fix.in` is `workbook`): fix the problem that the other diagnostic of this entry reports, or save the workbook again from Excel
