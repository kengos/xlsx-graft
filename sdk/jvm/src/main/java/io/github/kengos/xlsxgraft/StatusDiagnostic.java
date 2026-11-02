// Turns a C ABI status other than XLSX_GRAFT_OK into a Diagnostic, for a
// status the engine keeps no failure of: one with no book to keep it (an
// open), or after which its diagnostics cannot be read (a panic).
//
// The statuses that refuse a sheet are named as `xlsx-graft read` names them
// in its end line's `error.code`; the book's diagnostics say them in full,
// naming the sheet. The others name the status codes of
// engine/capi/include/xlsx_graft.h.

package io.github.kengos.xlsxgraft;

final class StatusDiagnostic {

  static final int OK = 0;
  static final int PANIC = 2;
  static final int IO = 3;
  static final int INVALID_ARG = 4;

  // The code and message of each status from 1, in the header's order.
  private static final String[][] CODES = {
    {"null_arg", "the engine was passed a null pointer"},
    {"panic", "the engine panicked; the call did not complete"},
    {"io", "the host failed to open, read or write a file"},
    {"invalid_arg", "the engine was passed text that is not UTF-8"},
    {"workbook_unreadable", "the workbook cannot be read; the diagnostics say why"},
    {"sheet_not_found", "no such sheet"},
    {"sheet_ambiguous", "more than one sheet has the name, letter case aside"},
    {"sheet_not_worksheet", "the sheet is not a worksheet"},
    {"sheet_unreadable", "the sheet cannot be read; the diagnostics say why"},
    {"closed", "the rows' book was closed"},
  };

  private StatusDiagnostic() {}

  /** The diagnostic for {@code status}, which a C ABI function returned. */
  static Diagnostic of(int status) {
    return of(status, null);
  }

  /**
   * The diagnostic for {@code status}, saying {@code message} in place of the status's own message
   * when it is neither null nor empty (why the host failed).
   */
  static Diagnostic of(int status, String message) {
    String code;
    String known;
    if (status >= 1 && status <= CODES.length) {
      code = CODES[status - 1][0];
      known = CODES[status - 1][1];
    } else {
      code = "unknown_status";
      known = "the engine returned status " + status + ", which this package does not know";
    }
    return error(code, message == null || message.isEmpty() ? known : message);
  }

  /** An {@code "error"} diagnostic of the package's own, not from the engine's diagnostics. */
  static Diagnostic error(String code, String message) {
    return new Diagnostic(code, "error", message);
  }
}
