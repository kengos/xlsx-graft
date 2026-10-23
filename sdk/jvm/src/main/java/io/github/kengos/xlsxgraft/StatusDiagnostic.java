// Turns a C ABI status other than XLSX_GRAFT_OK into a Diagnostic.
//
// The codes are provisional: they name the status codes of
// engine/capi/include/xlsx_graft.h until the engine reports diagnostics of its
// own, which replace them.

package io.github.kengos.xlsxgraft;

final class StatusDiagnostic {

  static final int OK = 0;

  private StatusDiagnostic() {}

  /** The diagnostic for {@code status}, which a C ABI function returned. */
  static Diagnostic of(int status) {
    switch (status) {
      case 1:
        return new Diagnostic("null_arg", "the engine was passed a null pointer");
      case 2:
        return new Diagnostic("panic", "the engine panicked; the call did not complete");
      default:
        return new Diagnostic(
            "unknown_status",
            "the engine returned status " + status + ", which this package does not know");
    }
  }
}
