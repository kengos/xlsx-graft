# xlsx-graft

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
  `target_invalid`). No host reads a template yet: verifying is reached
  only through the engine's Rust API, which takes the lists from its
  caller.
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
  - What it carries over from a template is never rewritten: it is
    reported as verifying would report it.
  - Valid under ECMA-376 does not mean that Google Sheets, LibreOffice or
    Numbers opens it: name them in `targets:` to have it checked.

## License

Licensed under any of the following, at your option (`MIT OR Apache-2.0 OR BSD-3-Clause`):

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- BSD 3-Clause license ([LICENSE-BSD](LICENSE-BSD))

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you shall be licensed as above, without any additional terms or conditions.
