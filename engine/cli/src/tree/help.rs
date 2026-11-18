//! The long help of `tree`: what it prints in each format and how it
//! exits. The JSON it describes is a contract, so the help is where a
//! script's author reads it, and `docs/sdk.md` points here.

/// `tree --help`'s description.
pub(crate) const LONG_ABOUT: &str = "\
List the parts of a workbook (an xlsx package) and the relationships between
them, as a tree from the package: what the workbook holds and what refers to
what, without opening it in Excel. The C ABI, the SDKs and the MCP server give
the same view.";

/// `tree --help`'s text after the options.
pub(crate) const AFTER_LONG_HELP: &str = r##"Text (the default) is for a person and may change; it is not a contract. It is
a tree of lines drawn with box characters: the package, then under it each
relationship with the part it targets, and a part's own relationships under
the part. A part the tree above already showed is marked (above), an external
target (external), a part whose relationships were not all kept (some
relationships not listed), and a part first found past 32 levels (continued
below), shown again with what it targets after the tree (continued). A part no
relationship targets is shown after the tree, marked not referenced. The
diagnostics follow, then a line of the number of parts, the counts of the
diagnostics and whether the view is ok. All of it goes to stdout.

JSON (--format json) is one line, a contract:
{"ok": BOOL, "error": ERROR, "diagnostics": [...], "complete": BOOL,
 "relationshipsKnown": BOOL, "relationships": [REL, ...], "parts": [PART, ...]}

  ok           error is null, and no diagnostic is an error
  error        null, or {"code": "io", "message": TEXT} when the file cannot
               be opened, is not a regular file, or stopped being readable
               while it was read. What is wrong inside the file (not a zip, a
               part that cannot be read) is a diagnostic. parts is empty
               when the package could not be opened (not a zip, say), and
               when error is io
  diagnostics  every problem found, as every host prints them
  complete     false when a Relationships part was not read, or a
               relationship was not kept, that may have targeted a part
  relationshipsKnown
               false when the package's own relationships may include one
               that is not listed
  relationships
               the package's own relationships, in the order of their
               Relationships part
  parts        every part, in the order of its zip entry (Relationships parts
               are not listed: their relationships are on the part they
               belong to)

A PART is {"name": TEXT, "contentType": TEXT or null,
"contentTypeKnown": BOOL, "size": N, "compressedSize": N, "readable": BOOL,
"referenced": BOOL or null, "relationshipsKnown": BOOL,
"relationships": [REL, ...]}:

  contentTypeKnown
               false when a null contentType does not mean the part has none
  size, compressedSize
               bytes, as the zip's central directory declares them for the
               entry, inflated and as stored
  readable     whether the package reader accepted the part's zip entry
  referenced   true when a listed relationship targets the part; otherwise
               false when complete is true, and null when it is false
  relationshipsKnown
               false when the part's relationships may include one that is
               not listed

A REL is {"id": TEXT, "type": TEXT, "target": {"part": TEXT} or
{"external": TEXT}}: the relationship's Id and Type as written, and a part of
the package or a resource outside it.

A quoted value (a name, an id, a type, a content type or an external target)
is quoted from the workbook and tidied for display: control characters,
bidirectional formatting characters, the line and paragraph separators
(U+2028, U+2029) and the tag characters (U+E0000 to U+E007F) are removed, and
a value over 200 characters is cut and ends in "…". Two different values may
therefore be written the same. The values are for display, not for comparing
or for using again as a path, and no command prints them exactly.

Like verify, tree prints its view whatever happened: the object is on stdout
whether the status is 0 or 1, and once it is printed stderr stays empty.

Exit status: 0 when ok. 1 when not ok, including when the file cannot be
opened (error says why). 2 for a command line not taken (clap's message on
stderr, and no object). When stdout closes early, nothing more is said, and
the status is 0.

Examples, from the repository's root:
  xlsx-graft tree fixtures/table.xlsx
  xlsx-graft tree fixtures/external-link.xlsx --format json"##;
