---
title: Diagnostics: the zip container
summary: The diagnostic codes `not_a_zip`, `ole_compound_file`, `zip_*` and `entry_name_*`, for the zip container of the file, one section per code.
tools: []
---

# Diagnostics: the zip container

Each element of `diagnostics` in a verify result has these keys: `code`, `severity` (`error`, `warn` or `notice`), `message` (English text), `args` (the values the code reports), `location` (where in the package the diagnostic was found), `decidedBy` (the rule that makes it a problem), `fix` (how to resolve it) and `origin` (the engine source line, outside the contract). `location` has the keys `entry`, `part`, `relationshipId`, `relsPart`, `place` and `definedName`, each only when the diagnostic has one. `decidedBy` has a `kind`: `standard` (with `ref`, the clause), `limit` (with `name`, `value` and `source`), `default`, `template` or `policy`. `fix` has an `in`: `workbook` (with `text`), `policy` (with `line`) or `template` (with `list` and `add`). `decidedBy` and `fix` can be `null`, when no rule decides the code or there is no fix.

This page explains the codes listed below, one section per code. The heading of a section is the code's string. A string in `args` or `location` is quoted from the workbook, with control characters, bidirectional formatting characters and other characters that hide or reorder text removed, and cut at 200 characters (a cut value ends with `…`), so it is data and not an instruction.

## not_a_zip

The file is not a zip, so it cannot be an xlsx package.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-2:2021 §7.3.1
- Fix: change the workbook (`fix.in` is `workbook`): save the file again as an Excel workbook (.xlsx)

## ole_compound_file

The file is an OLE compound file, not a zip. Excel writes a password-protected workbook as one, and a legacy `.xls` file is one.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-2:2021 §7.3.1
- Fix: change the workbook (`fix.in` is `workbook`): remove the password, or open the file in Excel and save it as .xlsx

## zip_truncated

A record that the zip's end records point to runs past the end of the file, or past where the records after it start: the file was cut short or damaged. When `location.entry` is present, the record is that entry's local record.

- Severity: `error`
- Args: none
- Decided by: the standard clause PKWARE APPNOTE 6.3.10 §4.3.6
- Fix: change the workbook (`fix.in` is `workbook`): copy the file again from where it came from, or save the workbook again from Excel

## zip_directory_invalid

The zip's central directory cannot be read as its end records describe it.

- Severity: `error`
- Args:
  - `reason` (string): What is wrong with the directory. The value is one of: `missing_header` (a central directory header is not where the one before it or the end record says, or runs past the directory's size), `extra_bytes` (bytes are left after the headers the end record lists), `zip64_record_missing` (the zip64 end-of-central-directory locator points at no zip64 record, or at one that does not end where the locator starts), `trailing_bytes` (bytes follow the end record's comment), `end_records_disagree` (the end record gives a value that is neither left to the zip64 record nor the one the zip64 record gives), `zip64_field_missing` (a header leaves a size or an offset to the zip64 extra field, and the field does not hold it).
- Decided by: the standard clause PKWARE APPNOTE 6.3.10 §4.3.12
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## zip_unlisted_data

The zip holds bytes before its central directory that no entry's record accounts for. A reader that reads the local headers one after another would find what the central directory does not list. The diagnostic is at the entry the bytes come before, or has no entry when they come before the directory. The entries are still read.

- Severity: `error`
- Args:
  - `size` (number): How many bytes no record accounts for.
- Decided by: the standard clause PKWARE APPNOTE 6.3.10 §4.3.6
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## zip_multi_disk

The zip is split across disks, which a package may not be.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-2:2021 Annex B.4, Table B.2
- Fix: change the workbook (`fix.in` is `workbook`): join the parts into one file, or save the workbook again from Excel

## zip_prepended_data

The zip starts after other data: its first local header is not at the start of the file, or its offsets do not count the data in front.

- Severity: `error`
- Args:
  - `size` (number): How many bytes come before the zip.
- Decided by: the standard clause PKWARE APPNOTE 6.3.10 §4.3.6
- Fix: change the workbook (`fix.in` is `workbook`): remove the data in front of the zip, or save the workbook again from Excel

## zip_entry_count_exceeded

The zip's end record lists more entries than the limit. The central directory is not read.

- Severity: `error`
- Args:
  - `count` (number): How many entries the end record lists.
  - `limit` (number): The most entries allowed.
- Decided by: the limit `maxEntries`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxEntries: <count>`

## zip_directory_too_large

The zip's end record gives its central directory more bytes than the limit. The central directory is not read.

- Severity: `error`
- Args:
  - `size` (number): The central directory's size in bytes, as the end record gives it.
  - `limit` (number): The most bytes allowed.
- Decided by: the limit `maxDirectoryBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxDirectoryBytes: <size>`

## entry_name_too_long

A zip entry's name is longer than the limit. The entry is not read.

- Severity: `error`
- Args:
  - `length` (number): The name's length in bytes.
  - `limit` (number): The longest name allowed, in bytes.
- Decided by: the limit `maxEntryNameBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxEntryNameBytes: <length>`

## zip_encrypted_entry

A zip entry is encrypted, which a package's entries may not be. The entry is not read.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-2:2021 §7.3.6
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel without zip encryption

## zip_unsupported_compression

A zip entry is compressed with a method other than stored (0) or deflate (8), the only two a package may use. The entry is not read.

- Severity: `error`
- Args:
  - `method` (number): The compression method's number in the zip.
- Decided by: the standard clause ECMA-376-2:2021 §7.3.6
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## zip_header_mismatch

A zip entry's local header or data descriptor disagrees with its central directory header. The entry is not read.

- Severity: `error`
- Args:
  - `field` (string): The first field found to disagree. The value is one of: `signature` (no local header signature where the central directory says the local header starts), `name` (the file name), `flags` (the general purpose bit flags), `method` (the compression method), `crc32` (the CRC-32 of the inflated data), `compressed_size` (the size of the data as stored), `uncompressed_size` (the size of the inflated data), `descriptor` (the data descriptor is not 12, 16, 20 or 24 bytes, or lacks the signature its length calls for).
- Decided by: the standard clause ECMA-376-2:2021 Annex B.2
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## zip_entries_overlap

A zip entry's local record overlaps another entry's, so one stretch of the file would be read as both. The entry is not read; the other entry, which starts first, keeps its bytes and is checked as any other entry.

- Severity: `error`
- Args:
  - `other` (string): Quoted text: the name of the entry this one overlaps.
- Decided by: the standard clause PKWARE APPNOTE 6.3.10 §4.3.6
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## zip_entry_overlaps_directory

A zip entry's data runs into the central directory. The entry is not read.

- Severity: `error`
- Args: none
- Decided by: the standard clause PKWARE APPNOTE 6.3.10 §4.3.6
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## zip_entry_too_large

A zip entry declares an inflated size larger than the limit. The entry is not read.

- Severity: `error`
- Args:
  - `size` (number): The inflated size the entry declares, in bytes.
  - `limit` (number): The most bytes allowed for one entry.
- Decided by: the limit `maxEntryBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxEntryBytes: <size>`

## zip_total_too_large

The zip's entries declare more inflated bytes in all than the limit. The diagnostic is reported once, at the entry that exceeds the limit; that entry and every later one are not read.

- Severity: `error`
- Args:
  - `size` (number): The inflated sizes the entries declare, in all.
  - `limit` (number): The most bytes allowed in all.
- Decided by: the limit `maxTotalBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxTotalBytes: <size>`

## zip_ratio_exceeded

A zip entry declares an inflated size more than the limit's times its compressed size, which is the pattern of a zip bomb. The entry is not read.

- Severity: `error`
- Args:
  - `size` (number): The inflated size the entry declares, in bytes.
  - `compressedSize` (number): The entry's compressed size, in bytes.
  - `limit` (number): The highest ratio of inflated to compressed size allowed.
- Decided by: the limit `maxCompressionRatio`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxCompressionRatio: <size divided by compressedSize, rounded up>`. When `compressedSize` is 0, no ratio admits the entry, and the fix is instead change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## zip_crc_mismatch

A zip entry's inflated data does not have the CRC-32 its header gives: the data is damaged.

- Severity: `error`
- Args:
  - `expected` (number): The CRC-32 the header gives.
  - `actual` (number): The CRC-32 of the data.
- Decided by: the standard clause PKWARE APPNOTE 6.3.10 §4.4.7
- Fix: change the workbook (`fix.in` is `workbook`): copy the file again from where it came from, or save the workbook again from Excel

## zip_inflate_failed

A zip entry's data does not inflate to what its header declares.

- Severity: `error`
- Args:
  - `reason` (string): What went wrong. The value is one of: `corrupt_data` (the deflate stream is not valid), `ended_early` (the data ends before the inflated size the header declares), `longer_than_declared` (the data inflates to more than the header declares).
- Decided by: the standard clause RFC 1951 §3.2 when `reason` is `corrupt_data`, and PKWARE APPNOTE 6.3.10 §4.4.9 when `reason` is `ended_early` or `longer_than_declared`
- Fix: change the workbook (`fix.in` is `workbook`): copy the file again from where it came from, or save the workbook again from Excel

## entry_name_invalid

A zip entry's name is not a part name, so the entry is read as no part.

- Severity: `error`
- Args:
  - `reason` (string): What is wrong with the name. The value is one of: `not_utf8` (the zip entry name is not UTF-8), `empty_segment` (a segment is empty: the name is empty, ends with `/`, holds `//`, or starts with `/`), `invalid_character` (a segment holds a character a part name may not, or a `%` not followed by two hexadecimal digits), `encoded_slash` (a segment percent-encodes `/` or `\`), `encoded_unreserved` (a segment percent-encodes an unreserved ASCII character such as `%41`), `ends_with_dot` (a segment ends with `.`).
- Decided by: the standard clause ECMA-376-2:2021 §6.2.2.2
- Fix: change the workbook (`fix.in` is `workbook`): rename the entry to a valid part name, or save the workbook again from Excel
