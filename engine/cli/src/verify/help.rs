//! The long help of `verify`: what it prints in each format and how it
//! exits. The JSON it describes is a contract, so the help is where a
//! script's author reads it, and `docs/sdk.md` points here.

/// `verify --help`'s description.
pub(crate) const LONG_ABOUT: &str = "\
Verify a workbook: check it against the host's policy (--policy) and the
caller's rules (--target, --best-effort, --allow-function, --allow-part,
--fail-on), without opening it in Excel. The C ABI and the MCP server run the
same check.";

/// `verify --help`'s text after the options.
pub(crate) const AFTER_LONG_HELP: &str = r##"Text (the default) is for a person and may change: the diagnostics under an
error, warn and notice heading, a line each with its fix under it, then a line
of the counts and whether the workbook is ok. All of it goes to stdout.

JSON (--format json) is one line, a contract:
{"ok": BOOL, "failOn": LEVEL, "counts": {"error": N, "warn": N, "notice": N},
 "error": ERROR, "diagnostics": [...]}

  ok           error is null, and no diagnostic is at least as severe as failOn
  failOn       "error" or "warn": the stricter of the policy's failOn and
               --fail-on. With "warn" a warning, such as a --best-effort
               version the workbook does not work in, makes ok false
  counts       how many of each severity diagnostics lists
  error        null, or {"code": "io", "message": TEXT} when the workbook or
               the policy file cannot be opened or read
  diagnostics  every problem found, as every host prints them. Each fault of the
               policy or the rules is a diagnostic (policy_invalid,
               rules_invalid, target_invalid); every fault is listed in one
               run, and the workbook is not opened then. When the policy file
               cannot be read, the rules are not checked: error is set and
               diagnostics is empty

Unlike sheets and read, verify prints its result whatever happened: the object
is on stdout whether the status is 0, 1 or 2, and once it is printed stderr
stays empty.

Exit status: 0 when ok. 1 when not ok, or when the workbook or the policy file
cannot be opened or read (error says why). 2 when the policy or the rules are
not valid (diagnostics say why), or for a command line not taken (clap's
message on stderr, and no object). When stdout closes early, nothing more is
said, and the status is 0.

The policy (--policy FILE) is a JSON object of at most 1 MiB, such as
  {"maxFormulaBytes": 32768, "failOn": "warn", "denyScheduled": true,
   "forbid": {"functions": [{"name": "HYPERLINK", "reason": "..."}],
              "parts": [{"kind": "ole", "reason": "..."}]}}
A limit's key is the one a diagnostic's "fix (policy)" line names, such as
maxFormulaBytes; a key the policy does not know is policy_invalid.

Examples, from the repository's root:
  xlsx-graft verify fixtures/table.xlsx
  xlsx-graft verify fixtures/macro.xlsm --format json
  xlsx-graft verify fixtures/macro.xlsm --allow-part macro"##;
