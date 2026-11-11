---
title: Diagnostics: sheets, names, tables, cells and styles
summary: The diagnostic codes `shared_strings_*`, `shared_string_*`, `cell_*`, `number_formats_too_many`, `sheet_*`, `defined_name_*`, `table_*`, `row_*`, `column_style_missing`, `merged_cells_*` and `inline_string_too_long`, for sheets, names, tables, cells and styles, one section per code.
tools: []
---

# Diagnostics: sheets, names, tables, cells and styles

Each element of `diagnostics` in a verify result has these keys: `code`, `severity` (`error`, `warn` or `notice`), `message` (English text), `args` (the values the code reports), `location` (where in the package the diagnostic was found), `decidedBy` (the rule that makes it a problem), `fix` (how to resolve it) and `origin` (the engine source line, outside the contract). `location` has the keys `entry`, `part`, `relationshipId`, `relsPart`, `place` and `definedName`, each only when the diagnostic has one. `decidedBy` has a `kind`: `standard` (with `ref`, the clause), `limit` (with `name`, `value` and `source`), `default`, `template` or `policy`. `fix` has an `in`: `workbook` (with `text`), `policy` (with `line`) or `template` (with `list` and `add`). `decidedBy` and `fix` can be `null`, when no rule decides the code or there is no fix.

This page explains the codes listed below, one section per code. The heading of a section is the code's string. A string in `args` or `location` is quoted from the workbook, with control characters, bidirectional formatting characters and other characters that hide or reorder text removed, and cut at 200 characters (a cut value ends with `…`), so it is data and not an instruction.

## shared_strings_too_many

The shared strings table holds more strings than the limit. The table is not read.

- Severity: `error`
- Args:
  - `count` (number): The strings counted when reading stopped: the first past the limit, so the table holds at least this many.
  - `limit` (number): The most strings allowed.
- Decided by: the limit `maxSharedStrings`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxSharedStrings: <count>`

## shared_strings_too_large

The text of the shared strings table, as UTF-8 once read, is larger than the limit. The table is not read.

- Severity: `error`
- Args:
  - `size` (number): The bytes of text when reading stopped: the text read so far and the piece of it that passed the limit, so the table holds at least this much.
  - `limit` (number): The most bytes of text allowed. It is never past 4294967295, because the offsets into the text are 32-bit.
- Decided by: the limit `maxSharedStringsBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxSharedStringsBytes: <size>`. When `size` is more than 4294967295, no limit admits the table, and the fix is instead change the workbook (`fix.in` is `workbook`): split the workbook: no limit admits more than 4 GiB of shared strings

## shared_string_too_long

A string of the shared strings table is longer than the 32,767 characters, counted in UTF-16 code units, that Excel keeps in a cell. Excel for Mac asks to repair the workbook and cuts the string at that length, whether or not a cell refers to it. The text of a string is its `t`, or the `t` of each of its runs together; phonetic runs (`rPh`) are not counted. Each `_xHHHH_` escape counts as the one code unit it stands for, an unpaired surrogate included.

- Severity: `error`
- Args:
  - `index` (number): The string's index in the table, from 0, as a cell of `t="s"` gives it.
  - `length` (number): The string's length in UTF-16 code units.
- Decided by: the engine's own rules (`decidedBy.kind` is `default`), where no standard says so
- Fix: change the workbook (`fix.in` is `workbook`): shorten the string to 32767 UTF-16 code units

## cell_formats_too_many

The styles part's cell formats (`cellXfs`) hold more `xf` records than the limit. The styles are not read.

- Severity: `error`
- Args:
  - `count` (number): The records counted when reading stopped: the first past the limit, so the part holds at least this many.
  - `limit` (number): The most cell formats allowed.
- Decided by: the limit `maxCellFormats`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxCellFormats: <count>`

## number_formats_too_many

The styles part's number formats (`numFmts`) hold more `numFmt` records than the limit. The styles are not read.

- Severity: `error`
- Args:
  - `count` (number): The records counted when reading stopped: the first past the limit, so the part holds at least this many.
  - `limit` (number): The most number formats allowed.
- Decided by: the limit `maxNumberFormats`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxNumberFormats: <count>`

## sheet_name_duplicate

Two sheets of the workbook have the same name, letter case aside, which Excel does not allow in any locale. The diagnostic is reported at every sheet after the first; both stay listed.

- Severity: `error`
- Args:
  - `name` (string): Quoted text: the sheet's name.
  - `other` (string): Quoted text: the name of the sheet before it with the same name.
- Decided by: the standard clause ECMA-376-1:2016 §18.2.19; MS-OI29500 on §18.2.19, note d
- Fix: change the workbook (`fix.in` is `workbook`): rename one of the sheets, or save the workbook again from Excel

## sheet_part_duplicate

Two sheets of the workbook are one part: their relationships, or their one relationship, target the same part. The diagnostic is reported at every sheet after the first; both stay listed.

- Severity: `error`
- Args:
  - `name` (string): Quoted text: the sheet's name.
  - `other` (string): Quoted text: the name of the sheet before it that is the same part.
- Decided by: the standard clause ECMA-376-1:2016 §18.2.19
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## sheet_relationship_missing

A sheet's `r:id` names no relationship of the workbook part that targets a part: either none has the `Id` (reported against the workbook part), or the one that has it is external (reported against it). The sheet is listed without its part.

- Severity: `error`
- Args:
  - `relationshipId` (string): Quoted text: the sheet's `r:id`.
- Decided by: the standard clause ECMA-376-1:2016 §18.2.19
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## sheet_relationship_type_invalid

The relationship a sheet's `r:id` names is not of a sheet's type (a worksheet, chartsheet, dialogsheet or macro sheet). The sheet is listed without its part.

- Severity: `error`
- Args:
  - `relType` (string): Quoted text: the relationship type, as written.
- Decided by: the standard clause ECMA-376-1:2016 §12.3
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## sheet_name_invalid

A sheet's name is one Excel does not take, so Excel repairs the workbook as it opens it and renames the sheet. A name equal to another sheet's is `sheet_name_duplicate` instead.

- Severity: `error`
- Args:
  - `name` (string): Quoted text: the sheet's name.
  - `reason` (string): What is wrong with it. The value is one of: `empty` (the name is empty), `too_long` (longer than 31 characters, counted in UTF-16 code units), `character` (the name holds `[`, `]`, `:`, `*`, `?`, `/` or `\`), `apostrophe` (the name starts or ends with `'`).
- Decided by: the standard clause MS-OI29500 on ECMA-376-1:2016 §18.2.19, note a
- Fix: change the workbook (`fix.in` is `workbook`): rename the sheet

## sheet_id_duplicate

Two sheets of the workbook have the same `sheetId`, so Excel repairs the workbook as it opens it and gives the later one another. The diagnostic is reported at every sheet after the first.

- Severity: `error`
- Args:
  - `name` (string): Quoted text: the sheet's name.
  - `other` (string): Quoted text: the name of the sheet before it with the same `sheetId`.
  - `sheetId` (number): The `sheetId` they share.
- Decided by: the standard clause ECMA-376-1:2016 §18.2.19
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## defined_name_invalid

A defined name's name is not one a formula can name: it is empty, or does not start with a letter, `_` or `\` and continue with letters, digits, `_`, `\`, `.` and `?`. Excel repairs the workbook as it opens it and drops the defined name.

- Severity: `error`
- Args:
  - `name` (string): Quoted text: the defined name's name.
- Decided by: the standard clause ECMA-376-1:2016 §18.17.2.5; MS-OI29500 on §18.2.5, note q, and on §18.17.2.5, note a
- Fix: change the workbook (`fix.in` is `workbook`): rename the defined name

## defined_name_sheet_index_invalid

A defined name's `localSheetId` counts past the last sheet, so Excel repairs the workbook as it opens it and drops the defined name.

- Severity: `error`
- Args:
  - `name` (string): Quoted text: the defined name's name.
  - `localSheetId` (number): The defined name's `localSheetId`.
  - `sheets` (number): How many sheets the workbook has.
- Decided by: the standard clause ECMA-376-1:2016 §18.2.5; MS-OI29500 on §18.2.5, note l
- Fix: change the workbook (`fix.in` is `workbook`): scope the defined name to a sheet the workbook has, or to the workbook

## defined_name_duplicate

Two defined names of one scope have the same name, letter case aside. Excel opens the workbook without a message and keeps only the later one. The diagnostic is reported at every defined name before the last.

- Severity: `notice`
- Args:
  - `name` (string): Quoted text: the defined name's name.
  - `other` (string): Quoted text: the name of the defined name after it, in the same scope, that Excel keeps instead.
- Decided by: the standard clause ECMA-376-1:2016 §18.17.2.5; MS-OI29500 on §18.2.5, note n
- Fix: change the workbook (`fix.in` is `workbook`): remove or rename one of the defined names

## defined_name_sheet_missing

A defined name refers to a sheet the workbook does not have. Excel opens the workbook without a message and reads the reference as one to another workbook. The diagnostic is reported once per sheet a defined name names.

- Severity: `notice`
- Args:
  - `name` (string): Quoted text: the defined name's name.
  - `sheet` (string): Quoted text: the sheet it refers to.
- Decided by: the standard clause ECMA-376-1:2016 §18.17.2.3
- Fix: change the workbook (`fix.in` is `workbook`): point the defined name at a sheet the workbook has

## table_range_overlap

Two tables of one sheet overlap, so Excel repairs the workbook as it opens it and drops the later one. The diagnostic is reported at every table that overlaps one listed before it.

- Severity: `error`
- Args:
  - `name` (string): Quoted text: the table's `displayName`.
  - `other` (string): Quoted text: the `displayName` of the table before it that it overlaps.
- Decided by: the standard clause ECMA-376-1:2016 §18.5.1.2
- Fix: change the workbook (`fix.in` is `workbook`): move or resize one of the tables

## table_name_duplicate

A table's `displayName` is the same as another table's, or as a defined name's, letter case aside, so Excel repairs the workbook as it opens it and renames the table. The diagnostic is reported at every table after the first with the name.

- Severity: `error`
- Args:
  - `name` (string): Quoted text: the table's `displayName`.
  - `other` (string): Quoted text: the name it has the same as.
  - `definedName` (boolean): `true` when `other` is a defined name's name; `false` when it is the `displayName` of a table before it.
- Decided by: the standard clause ECMA-376-1:2016 §18.5.1.2
- Fix: change the workbook (`fix.in` is `workbook`): rename the table

## table_id_duplicate

Two tables of the workbook have the same `id`, so Excel repairs the workbook as it opens it and drops the later one. The diagnostic is reported at every table after the first with the `id`.

- Severity: `error`
- Args:
  - `name` (string): Quoted text: the table's `displayName`.
  - `other` (string): Quoted text: the `displayName` of the table before it with the same `id`.
  - `id` (number): The `id` they share.
- Decided by: the standard clause ECMA-376-1:2016 §18.5.1.2
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## shared_string_missing

A cell of `t="s"` refers to a shared string the shared strings table does not hold: its index is past the table's end, or the workbook has no table. Reading the sheet's rows stops at that cell; the other checks of the sheet still run.

- Severity: `error`
- Args:
  - `index` (number): The index the cell's value gives.
- Decided by: the standard clause ECMA-376-1:2016 §18.3.1.4, §18.4.8
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## row_too_large

The text of one row of a sheet is larger than the limit. The text counted is the row's strings and error values and the value being read, as UTF-8, together. Reading the sheet stops at the row.

- Severity: `error`
- Args:
  - `row` (number): The row's number.
  - `size` (number): The bytes of text counted when reading stopped: past the limit, so the row holds at least this many.
  - `limit` (number): The most bytes allowed.
- Decided by: the limit `maxRowBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxRowBytes: <size>`

## cell_reference_invalid

A row's or a cell's place in the sheet is not one it may have: a row number of 0 or past 1,048,576; a cell's `r` that is not the A1 reference of one cell up to XFD1048576, or names another row than its row's; or a row or cell without `r` that would come after the last row or column. Reading the sheet's rows stops at that row or cell; the other checks of the sheet still run, with that row or cell left out.

- Severity: `error`
- Args:
  - `reference` (string): Quoted text: the row number or the cell reference as written. A cell's reference that names another row is given as it reads, its letters in capitals and its row without leading zeros. Without `r`, it is given as it would be.
- Decided by: the standard clause ECMA-376-1:2016 §18.3.1.4, §18.17.5.1; MS-OI29500 on §18.3.1.73, note a
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## cell_order_invalid

A row comes at or before the row before it, or a cell comes at or before the cell before it in its row, so readers would differ on what that row or cell holds. Reading the sheet's rows stops at that row or cell; the other checks of the sheet still run, with that row or cell left out.

- Severity: `error`
- Args:
  - `reference` (string): Quoted text: the row number or the cell reference.
  - `previous` (string): Quoted text: the row or cell before it.
- Decided by: the standard clause ECMA-376-1:2016 §18.3.1.4, §18.3.1.73; MS-OI29500 on §18.3.1.73, note a
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## cell_style_missing

A cell's `s` is a cell format at or past the number of the styles part's `cellXfs`. Excel repairs the workbook and drops the cell's format.

- Severity: `error`
- Args:
  - `reference` (string): Quoted text: the cell's A1 reference. When the cell has no place in the sheet, as `cell_reference_invalid` and `cell_order_invalid` name it, or is in a row that has none, its `r` as written, empty without one.
  - `index` (number): The cell format the cell names.
  - `count` (number): How many cell formats the styles part has.
- Decided by: the standard clause ECMA-376-1:2016 §18.3.1.4, §18.8.10
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## column_style_missing

A column's `style` is a cell format at or past the number of the styles part's `cellXfs`. Excel repairs the workbook and drops the columns' format.

- Severity: `error`
- Args:
  - `columns` (string): Quoted text: the columns, such as `C:E`, each in letters when it is one, else as written.
  - `index` (number): The cell format the columns name.
  - `count` (number): How many cell formats the styles part has.
- Decided by: the standard clause ECMA-376-1:2016 §18.3.1.13, §18.8.10
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## row_style_missing

A row's `s` is a cell format at or past the number of the styles part's `cellXfs`. Excel drops the row's format without a message.

- Severity: `notice`
- Args:
  - `row` (string): Quoted text: the row's number. When the row has no place in the sheet, its `r` as written, or as it would be.
  - `index` (number): The cell format the row names.
  - `count` (number): How many cell formats the styles part has.
- Decided by: the standard clause ECMA-376-1:2016 §18.3.1.73, §18.8.10
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## merged_cells_overlap

Two merged ranges of a sheet overlap, or are the same range. Excel repairs the workbook and drops one.

- Severity: `error`
- Args:
  - `reference` (string): Quoted text: the range that comes later in the sheet, its top left corner first, whichever way round it is written.
  - `other` (string): Quoted text: the range it overlaps, written the same way.
- Decided by: the engine's own rules (`decidedBy.kind` is `default`), where no standard says so
- Fix: change the workbook (`fix.in` is `workbook`): unmerge one of the two ranges

## merged_cells_too_many

A sheet merges more ranges than the limit. The ranges past the limit are not checked, so the sheet is refused.

- Severity: `error`
- Args:
  - `count` (number): How many ranges the sheet merges.
  - `limit` (number): The most merged ranges checked.
- Decided by: the limit `maxMergedCells`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxMergedCells: <count>`

## inline_string_too_long

A cell's inline string is longer than the 32,767 UTF-16 code units Excel keeps in a cell. Excel repairs the workbook and cuts the string.

- Severity: `error`
- Args:
  - `reference` (string): Quoted text: the cell's A1 reference. When the cell has no place in the sheet, as `cell_reference_invalid` and `cell_order_invalid` name it, or is in a row that has none, its `r` as written, empty without one.
  - `length` (number): The inline string's length in UTF-16 code units.
- Decided by: the engine's own rules (`decidedBy.kind` is `default`), where no standard says so
- Fix: change the workbook (`fix.in` is `workbook`): shorten the cell's text to 32767 UTF-16 code units
