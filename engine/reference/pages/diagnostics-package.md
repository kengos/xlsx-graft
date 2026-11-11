---
title: Diagnostics: XML, parts and relationships
summary: The diagnostic codes `xml_*`, `part_name_*`, `content_types_*`, `content_type_*`, `part_content_type_*`, `relationships_*`, `relationship_*`, `main_document_missing` and `part_unreferenced`, for XML, parts and relationships, one section per code.
tools: []
---

# Diagnostics: XML, parts and relationships

Each element of `diagnostics` in a verify result has these keys: `code`, `severity` (`error`, `warn` or `notice`), `message` (English text), `args` (the values the code reports), `location` (where in the package the diagnostic was found), `decidedBy` (the rule that makes it a problem), `fix` (how to resolve it) and `origin` (the engine source line, outside the contract). `location` has the keys `entry`, `part`, `relationshipId`, `relsPart`, `place` and `definedName`, each only when the diagnostic has one. `decidedBy` has a `kind`: `standard` (with `ref`, the clause), `limit` (with `name`, `value` and `source`), `default`, `template` or `policy`. `fix` has an `in`: `workbook` (with `text`), `policy` (with `line`) or `template` (with `list` and `add`). `decidedBy` and `fix` can be `null`, when no rule decides the code or there is no fix.

This page explains the codes listed below, one section per code. The heading of a section is the code's string. A string in `args` or `location` is quoted from the workbook, with control characters, bidirectional formatting characters and other characters that hide or reorder text removed, and cut at 200 characters (a cut value ends with `…`), so it is data and not an instruction.

## xml_malformed

A part's XML is not well-formed: the parser stopped at `offset`.

- Severity: `error`
- Args:
  - `offset` (number): The byte offset in the part's text, read as UTF-8 after any byte order mark: the start of the markup that holds the problem, or where the parser stopped.
  - `detail` (string): Quoted text: what the parser found at that offset, in its own English words.
- Decided by: the standard clause XML 1.0 (Fifth Edition) §2.1
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel or the tool that wrote it

## xml_doctype

A part's XML declares a document type (DOCTYPE), which package XML may not do: entity declarations in it are a denial-of-service vector.

- Severity: `error`
- Args:
  - `offset` (number): The byte offset of the declaration, counted as for `xml_malformed`.
- Decided by: the standard clause ECMA-376-2:2021 §6.2.5
- Fix: change the workbook (`fix.in` is `workbook`): remove the DOCTYPE declaration, or save the workbook again from Excel

## xml_encoding_unsupported

A part's XML is not in UTF-8 or UTF-16, the only encodings package XML may use, or does not decode as the one it is in.

- Severity: `error`
- Args:
  - `encoding` (string): Quoted text: the encoding the declaration names, or else the one the bytes were read as.
  - `reason` (string): Why the part cannot be read. The value is one of: `other_encoding` (the byte order mark or the XML declaration names an encoding other than UTF-8 or UTF-16), `declaration_mismatch` (the declaration names UTF-8 or UTF-16 but the bytes are the other one, or the other byte order), `invalid_utf8` (the bytes are read as UTF-8 and are not valid UTF-8), `invalid_utf16` (the bytes are read as UTF-16 and are not valid UTF-16: an odd length or an unpaired surrogate), `utf16_without_bom` (the bytes look like UTF-16 but start without the byte order mark that XML requires of UTF-16).
- Decided by: the standard clause ECMA-376-2:2021 §6.2.5
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel, which writes UTF-8

## xml_too_deep

A part's XML nests elements deeper than the limit.

- Severity: `error`
- Args:
  - `offset` (number): The byte offset of the first element past the limit, counted as for `xml_malformed`.
  - `depth` (number): The deepest nesting found; the root element is at depth 1. When the XML turns out to be malformed past the limit, the deepest nesting before that.
  - `limit` (number): The deepest nesting allowed.
- Decided by: the limit `maxXmlDepth`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxXmlDepth: <depth>`

## xml_too_many_attributes

One element of a part's XML has more attributes than the limit.

- Severity: `error`
- Args:
  - `offset` (number): The byte offset of the element, counted as for `xml_malformed`.
  - `count` (number): How many attributes the element has. This is the first element past the limit, so a later element may have more.
  - `limit` (number): The most attributes allowed on one element.
- Decided by: the limit `maxXmlAttributes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxXmlAttributes: <count>`

## xml_too_many_namespaces

A part's XML declares more namespaces in scope at once than the limit.

- Severity: `error`
- Args:
  - `offset` (number): The byte offset of the element whose declarations exceed the limit, counted as for `xml_malformed`.
  - `count` (number): How many namespace declarations are in scope at that element. This is the first element past the limit, so a later element may have more.
  - `limit` (number): The most declarations allowed in scope at once.
- Decided by: the limit `maxXmlNamespaces`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxXmlNamespaces: <count>`

## xml_namespace_too_long

A namespace declaration in a part's XML is longer, as written, than the limit.

- Severity: `error`
- Args:
  - `offset` (number): The byte offset of the element that declares it, counted as for `xml_malformed`.
  - `length` (number): The bytes of the declaration's value, as written.
  - `limit` (number): The most bytes a declaration's value may hold.
- Decided by: the limit `maxXmlNamespaceBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxXmlNamespaceBytes: <length>`

## xml_event_too_large

One event of a part's XML (a tag, a run of text, a CDATA section, a comment, a processing instruction, the XML declaration, a document type declaration or a reference) is longer than the limit. Reading stops one byte past the limit, so the event's whole length is not known.

- Severity: `error`
- Args:
  - `offset` (number): The byte offset where the event starts, counted as for `xml_malformed`.
  - `size` (number): The bytes of the event read when reading stopped, as UTF-8. The event holds at least this many.
  - `limit` (number): The most bytes one event may hold.
- Decided by: the limit `maxXmlEventBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxXmlEventBytes: <size>`

## xml_too_large

A part's XML is larger than the engine reads into memory.

- Severity: `error`
- Args:
  - `size` (number): The part's size in bytes: its length when in memory, else the size its zip entry declares, or the bytes read before stopping when that is larger.
  - `limit` (number): The most bytes allowed.
- Decided by: the limit `maxXmlBytes`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxXmlBytes: <size>`

## xml_schema_invalid

An XML part breaks its schema. The part is a package XML stream (the content types stream or a Relationships part), where an error makes only that element invalid, or the whole stream when its root is wrong; or the shared strings table or the styles, which are then not read at all.

- Severity: `error`
- Args:
  - `offset` (number): The byte offset of the element, counted as for `xml_malformed`.
  - `reason` (string): How the part breaks the schema. The value is one of: `unexpected_element` (an element the schema does not allow at that position: another root, an unknown child, or one in another namespace), `unexpected_attribute` (an attribute the element does not have), `missing_attribute` (a required attribute is not there), `missing_element` (a required child element is not there), `unexpected_text` (text other than white space where only elements may stand), `invalid_value` (an attribute's value is not one its type allows).
  - `name` (string): Quoted text: the element or attribute, as written.
- Decided by: the standard clause ECMA-376-2:2021 Annex C, ECMA-376-1:2016 Annex A
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## part_name_duplicate

Two zip entries name the same part, their names equal but for ASCII letter case. The diagnostic is reported at every entry after the first, and none of them is read as a part, since readers differ on which one they take.

- Severity: `error`
- Args:
  - `other` (string): Quoted text: the first entry with the name.
- Decided by: the standard clause ECMA-376-2:2021 §6.2.2.3
- Fix: change the workbook (`fix.in` is `workbook`): remove one of the entries, or save the workbook again from Excel

## part_name_derived

A part's name continues another part's name with more segments (`/a/b` beside `/a`). It is not read as a part; the other one is.

- Severity: `error`
- Args:
  - `other` (string): Quoted text: the part name it is derived from.
- Decided by: the standard clause ECMA-376-2:2021 §6.2.2.3
- Fix: change the workbook (`fix.in` is `workbook`): rename one of the parts, or save the workbook again from Excel

## content_types_missing

The package has no content types stream (`[Content_Types].xml`), so no part has a media type. The diagnostic is reported once, not at every part.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-2:2021 §7.3.7
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## content_types_duplicate_default

The content types stream has more than one `Default` for an extension, letter case aside. None of them applies.

- Severity: `error`
- Args:
  - `extension` (string): Quoted text: the extension, as the later `Default` spells it.
- Decided by: the standard clause ECMA-376-2:2021 §7.2.3.2.1
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## content_types_duplicate_override

The content types stream has more than one `Override` for a part name, letter case aside. None of them applies.

- Severity: `error`
- Args:
  - `partName` (string): Quoted text: the part name, as the later `Override` spells it.
- Decided by: the standard clause ECMA-376-2:2021 §7.2.3.2.1
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## content_types_part_name_invalid

An `Override`'s `PartName` is not a part name, so it matches no part.

- Severity: `error`
- Args:
  - `partName` (string): Quoted text: the `PartName`, as written.
  - `reason` (string): What is wrong with it. The value is one of: `missing_leading_slash` (the name does not start with `/`), `empty_segment` (a segment is empty), `invalid_character` (a segment holds a character a part name may not, or a `%` not followed by two hexadecimal digits), `encoded_slash` (a segment percent-encodes `/` or `\`), `encoded_unreserved` (a segment percent-encodes an unreserved ASCII character such as `%41`), `ends_with_dot` (a segment ends with `.`).
- Decided by: the standard clause ECMA-376-2:2021 §7.2.3.2.5
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## content_type_invalid

A media type in the content types stream does not follow RFC 7231 §3.1.1.1 without white space or comments. The `Default` or `Override` that gives it applies to no part.

- Severity: `error`
- Args:
  - `contentType` (string): Quoted text: the media type, as written.
- Decided by: the standard clause ECMA-376-2:2021 §7.2.3.2.4
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## part_content_type_missing

A part has no media type: no `Override` names it and no `Default` matches its extension.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-2:2021 §7.2.3.2.1
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## part_content_type_mismatch

A part's media type disagrees with its name: a Relationships part's name takes the Relationships media type, and no other part's name does.

- Severity: `error`
- Args:
  - `contentType` (string): Quoted text: the part's media type.
- Decided by: the standard clause ECMA-376-2:2021 §6.5.2.1
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## relationships_source_missing

A Relationships part's source part is not in the package. Its relationships are not read.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-2:2021 §6.5.2.3
- Fix: change the workbook (`fix.in` is `workbook`): remove the Relationships part, or save the workbook again from Excel

## relationships_too_many

A Relationships part has more relationships than the limit. None of them is read.

- Severity: `error`
- Args:
  - `count` (number): How many relationships the part has.
  - `limit` (number): The most relationships allowed in one part.
- Decided by: the limit `maxRelationships`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxRelationships: <count>`

## relationships_total_too_many

The package's Relationships parts hold more relationships in all than the limit. The diagnostic is reported once, at the part that exceeds the limit; that part and every later one are not read.

- Severity: `error`
- Args:
  - `count` (number): How many relationships the parts read so far hold, including that part's.
  - `limit` (number): The most relationships allowed in all.
- Decided by: the limit `maxTotalRelationships`, which a policy can change
- Fix: raise the limit in the policy (`fix.in` is `policy`); `fix.line` is `maxTotalRelationships: <count>`

## relationship_id_duplicate

Two relationships in one Relationships part share an `Id`. The diagnostic is reported at every one after the first; none of them is read.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-2:2021 §6.5.3.4
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## relationship_target_invalid

An internal relationship's target is not a relative reference that resolves to a part name. The relationship is not read.

- Severity: `error`
- Args:
  - `target` (string): Quoted text: the `Target`, as written.
  - `reason` (string): What is wrong with it. The value is one of: `has_scheme` (the reference has a scheme, or a colon in its first segment), `has_authority` (the reference starts with `//`), `empty_segment` (a segment is empty), `invalid_character` (a segment holds a character a part name may not, or a `%` not followed by two hexadecimal digits), `encoded_slash` (a segment percent-encodes `/` or `\`), `ends_with_dot` (a segment ends with `.`).
- Decided by: the standard clause ECMA-376-2:2021 §6.5.3.4
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## relationship_target_above_root

An internal relationship's target points above the package root with `..`. Resolving stays at the root, as RFC 3986 §5.2.4 has it. The relationship is read.

- Severity: `notice`
- Args:
  - `target` (string): Quoted text: the `Target`, as written.
- Decided by: the standard clause ECMA-376-2:2021 §6.4.3.5
- Fix: none (`fix` is `null`). The code has no fix.

## relationship_target_missing

An internal relationship's target names no part in the package.

- Severity: `error`
- Args:
  - `partName` (string): Quoted text: the part name the target resolves to.
- Decided by: the standard clause ECMA-376-2:2021 §6.5.3.4
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## relationship_rels_part

A relationship is from or to a Relationships part, which may have no relationships. The relationship is not read. A Relationships part whose source is a Relationships part is reported once, without a relationship, and none of its relationships is read.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-2:2021 §6.5.2.1
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## main_document_missing

The package relationships have no `officeDocument` relationship to a part, so the package has no main document and is not a workbook.

- Severity: `error`
- Args: none
- Decided by: the standard clause ECMA-376-1:2016 SpreadsheetML Workbook part
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel

## part_unreferenced

No relationship targets the part, so a reader that follows relationships cannot find it. A Relationships part is found by its name, so it is never reported.

- Severity: `notice`
- Args: none
- Decided by: none (`decidedBy` is `null`)
- Fix: none (`fix` is `null`). The code has no fix.

## relationship_type_duplicate

Two relationships of one type in one Relationships part target different parts where the part type allows only one, such as the workbook's shared strings table. Which one a reader takes is not known, so neither is read. The diagnostic is reported once, at the first relationship whose target differs from the first one's.

- Severity: `error`
- Args:
  - `relType` (string): Quoted text: the relationship type, as written.
- Decided by: the standard clause ECMA-376-1:2016 §12.3
- Fix: change the workbook (`fix.in` is `workbook`): save the workbook again from Excel
