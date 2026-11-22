//! The long help of `describe-function`: what it prints in each format and
//! how it exits. The JSON it describes is a contract, so the help is where a
//! script's author reads it.

/// `describe-function --help`'s description.
pub(crate) const LONG_ABOUT: &str = "\
Look a worksheet function up in the compatibility table verify uses, and print
the table's entries for it: the name Excel writes for it in a file and its
prefix, the version of each application that added it, the features it needs
as well, its safety and the sources. The MCP server gives the same JSON.";

/// `describe-function --help`'s text after the options.
pub(crate) const AFTER_LONG_HELP: &str = r##"Text (the default) is for a person and may change; it is not a contract. It
is a line with the name and its state, then, for a function the table has a
row of versions for, its file name, safety, a line for each application and
the features it needs, and the sources.

JSON (--format json) is one line, a contract. A function with a row of
versions:
{"name": TEXT, "known": "versioned", "fileName": TEXT or null,
 "prefix": TEXT or null, "support": SUPPORT, "requires": [REQUIRED, ...],
 "safety": {"class": CLASS, "danger": TEXT or null}, "sources": [TEXT, ...]}

A baseline function (SUM), which the table gives a safety and no row of
versions, has name, known ("baseline"), fileName, prefix, safety and sources.
A name the table does not list, which verify denies, has name and known
("unknown") only.

  name         the name without a leading _xlfn. and _xlws., in upper case
  fileName     prefix and name: how Excel writes the function in a file; null,
               with prefix, when the table does not say which prefix the
               function has
  prefix       "_xlfn.", "_xlfn._xlws." or "" for a function with none; null
               when the table does not say which prefix the function has
  support      {"excel": ENTRY, "excel-mac": ENTRY, "libreoffice": ENTRY,
               "google-sheets": ENTRY, "numbers": ENTRY, "onlyoffice": ENTRY}
  requires     the features the function needs as well, each
               {"feature": TEXT, "support": SUPPORT, "sources": [TEXT, ...]}
  safety       class is reviewed_safe, dangerous or unreviewed; danger names
               the kind of danger of a dangerous function, and is null
               otherwise
  sources      the URLs that the table's row is from. For a baseline function
               that ECMA-376 defines, the clause "ECMA-376-1:2016 §18.17.7".
               Empty for a baseline function that ECMA-376 does not define

An ENTRY is {"added": TEXT or false or null, "removed": TEXT or null,
"partial": BOOL, "fallback": TEXT or null, "notes": TEXT or null}. added is
the version that added the function ("2021"), the version that has it when the
versions before it are not known ("<=14.4"), a version that lacks it and a
later version that has it, the versions between them not known
(">2007,<=2016"), false when no version has it, and null when no source says.

describe-function prints its description for every name, and once it is printed
stderr stays empty.

Exit status: 0 for a function with a row of versions, a baseline function and
a name the table does not list. 2 for a command line not taken (clap's message
on stderr, and no description). When stdout closes early, nothing more is
said, and the status is 0.

Examples:
  xlsx-graft describe-function SUM
  xlsx-graft describe-function _xlfn.SEQUENCE --format json"##;
