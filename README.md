# xlsx-graft

## Memory

Reading a sheet or verifying a workbook holds one row at a time. The
shared strings table moves to a temporary file once it passes 32 MiB, so
peak memory does not grow with the row count: the CLI reads 1,000,000
rows of strings all different in about 35 MiB, and `make bench:memory`, a member of `make verify`,
fails a change that passes 64 MiB or grows more than 32 MiB from 100,000
to 1,000,000 rows. The figures, the Ruby gems they are compared with and
how to reproduce them: [docs/benchmarks.md](docs/benchmarks.md).

## Specifications

The engine is built against the editions below, and the code and the
diagnostics cite their clauses. This is not a certified conformance claim.

| Specification | Edition the code cites | What it covers here |
| --- | --- | --- |
| [ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/) Part 1 | 5th edition, 2016 (= [ISO/IEC 29500-1:2016](https://www.iso.org/standard/71691.html)) | SpreadsheetML, DrawingML, Strict |
| ECMA-376 Part 2 | 5th edition, 2021 (= [ISO/IEC 29500-2:2021](https://www.iso.org/standard/77818.html)) | the package: parts, content types, relationships |
| [ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/) Part 3 | | markup compatibility (`mc:Ignorable`, `mc:AlternateContent`) |
| ECMA-376 Part 4 | 5th edition, 2016 (= [ISO/IEC 29500-4:2016](https://www.iso.org/standard/71692.html)) | Transitional |
| [MS-OI29500](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/) | | where Office departs from ECMA-376 |
| [MS-XLSX](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-xlsx/) | | Excel's extensions (sparklines, dynamic arrays, …) |
| [MS-OFFMACRO2](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-offmacro2/) | | macro sheets, the VBA project, the macro-enabled workbook |
| [PKWARE APPNOTE](https://pkware.cachefly.net/webdocs/APPNOTE/APPNOTE-6.3.10.TXT) | 6.3.10 | the zip container |
| [RFC 1951](https://www.rfc-editor.org/rfc/rfc1951) | | Deflate |
| [XML 1.0](https://www.w3.org/TR/2008/REC-xml-20081126/) | Fifth Edition | the XML of every part |
| [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986), [RFC 3987](https://www.rfc-editor.org/rfc/rfc3987) | | part names and relationship targets |
| [RFC 7230](https://www.rfc-editor.org/rfc/rfc7230), [RFC 7231](https://www.rfc-editor.org/rfc/rfc7231), [RFC 7303](https://www.rfc-editor.org/rfc/rfc7303) | | media types in `[Content_Types].xml` |

RFC 7230 and 7231 are obsoleted by RFC 9110 and 9112; the code still cites
the older numbers.

ISO counts editions apart from Ecma (ISO's 4th edition is Ecma's 5th), so
an edition is matched by its year.

### Which rule wins

ECMA-376 is the base. In reading and verifying, where Excel departs from
it, the engine follows Excel, as MS-OI29500 describes; where Excel itself
departs from MS-OI29500, it follows what Excel was seen to do, and the code
is to say which Excel (Windows or Mac, and the version) and which MS-OI29500 note it
overrides (older comments do not all say it yet). What the writer makes
keeps to both ECMA-376 and MS-OI29500 instead; what it carries over from a
template is reported, not rewritten (below).

- **Reading** takes values as Excel reads them; it does not refuse
  everything Excel would refuse. It reads both Strict and Transitional.
  - The date system is 1904 when `date1904` says so; otherwise 1900 as the
    standard has it when `workbook@conformance` is `strict`, and 1900 with
    the day that never was, 1900-02-29, otherwise. `dateCompatibility` is
    not read, since Excel ignores it.
  - A negative serial is not a date. An error value the standard does not
    list is still an error.
  - Sheet names are compared after Unicode lowercasing, the same in every
    locale.
- **Viewing** shows what a workbook holds, without checking it: `tree` (the
  parts and their relationships) and `describe-sheet` (the MCP tool is
  `describe_sheet`: the cached values and types of the cells of one sheet,
  no formulas and no displayed values), in the CLI, the MCP server, the C ABI
  library, the Node.js addon and the SDKs (`Viewer`). A string quoted from
  the workbook is tidied for display; the exact values come from the CLI's
  `read` and `sheets` commands and the SDKs' readers (the MCP server has no
  tool for them).
- **Verifying** reports every problem, up to `maxDiagnostics`, each with
  what decided it (`decidedBy`) when anything citable did:
  - `standard`: a clause of any specification in the table above,
    MS-OI29500 as much as ECMA-376, in `ref`;
  - `limit`: a limit on untrusted input;
  - `default`: the engine's defaults, which deny some parts, functions and
    formulas, and what Excel repairs where no standard says;
  - `template`: the template's `targets:`, `bestEffort:` and `allow`
    lists;
  - `policy`: what the host forbids.

  Whether the applications can open what a workbook uses is read from the
  engine's compatibility table: a function or a feature that a version
  in `targets:` lacks, or is not confirmed to have, is an error, and one in
  `bestEffort:` a warning. A function the table has no row for, one
  Excel 2007 already had, is not checked. A template without `targets:`
  (or with an empty one) is taken to say
  `targets: [excel >= 2024, excel-mac >= 2024]`, whatever its
  `bestEffort:` says; a `bestEffort:` version the default already covers
  adds nothing (one that a written `targets:` covers stays an error,
  `target_invalid`). No host reads a template yet; the CLI, the SDKs and the
  MCP server's `verify` take the same lists from their caller, as `targets`
  and `bestEffort` of the call.
- **Writing** (not built yet; this is its policy):
  - It writes Transitional.
  - It honours the template's `targets:`, `bestEffort:` and `allow` as
    verifying does, the default for a template without `targets:`
    included.
  - It never drops what it was asked to write without saying so. Where a
    function or a feature the caller asked for is one a version in
    `targets:` lacks, or is not confirmed to have, and has an equivalent
    form those versions have (one that gives the same result), it may
    write that form and report it as a warning; where it has no such
    form, it refuses with an error. One that only a `bestEffort:` version lacks is written
    as asked, with a warning.
  - A name it makes from the caller's input (a defined name, a sheet name)
    breaks neither ECMA-376 nor MS-OI29500. The forms admitted only because
    Excel was seen to open them are for reading and verifying, never for
    writing.
  - A cell's value is written as what it is, never guessed from its text.
    A string is an inline string (`t="inlineStr"`, unless the caller puts
    it in the shared strings part, `t="s"`), so `=A1`, `00123` and
    `2024-01-01` stay text. A character that is not an XML Char, and an `_`
    that would start an escape, are written as `_xHHHH_`
    (ECMA-376-1:2016 §22.9.2.19). Text past 32,767 UTF-16 code units is cut
    there, at a character boundary, and the caller is told it was cut. A
    NaN, an infinity, an integer whose magnitude is past 2^53, a date or
    date-time with no serial date-time in the workbook's date system, and an
    elapsed time that cannot be read back are refused. A date or time is
    the serial date-time of its wall-clock value in the workbook's date
    system, with no time zone, and `t="d"` is never written.
  - What it carries over from a template is never rewritten: it is
    reported as verifying would report it.
  - Rewriting the references of formulas when the caller renames or deletes a
    sheet is part of what the caller asked for. It rewrites a closed list of
    places: defined names, and in worksheet, macrosheet, dialogsheet, chart
    and table parts a sheet's `f`, a data validation's `formula1` and
    `formula2`, a conditional format's `formula` and `xm:f`, a chart's `c:f`,
    and a table's `calculatedColumnFormula` and `totalsRowFormula`, and in a
    pivot cache definition the `worksheetSource@sheet`. A formula anywhere
    else that verify reads as a formula (any other part, such as a chartex
    part's `cx:f`, which is only reported, or a place such as `cfvo@val`,
    `sp@textlink` and a revision's `oldFormula`; not an external workbook's
    `definedName@refersTo`) is left as it is and reported in the
    `StructureReport` when it names the renamed or deleted sheet. A delete
    writes one `#REF!` in place of a prefix of the deleted sheet and the
    operand it qualifies (`Sheet1!A1:B2`, `Sheet1!A2#`; Excel saves
    `Sheet1!A2#` as `_xlfn.ANCHORARRAY(Sheet1!A2)`, which a delete of
    `Sheet1` writes as `_xlfn.ANCHORARRAY(#REF!)`, and the `#` form is
    written by other applications); a prefix with no operand (the token
    after it is none of a cell, a range, a table reference, a name, an
    error value, or a call with its `)`, as in `Sheet1!(A1)`, `Sheet1!1` or
    `Sheet1!A1:Sheet1!B2`, or a call whose `(` never closes) is left as
    written and reported, because it still names the deleted sheet.
    The report holds at most 16,384 references and counts the others.
  - Valid under ECMA-376 does not mean that Google Sheets, LibreOffice or
    Numbers opens it: name them in `targets:` to have it checked.

## License

Licensed under any of the following, at your option (`MIT OR Apache-2.0 OR BSD-3-Clause`):

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- BSD 3-Clause license ([LICENSE-BSD](LICENSE-BSD))

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you shall be licensed as above, without any additional terms or conditions.
