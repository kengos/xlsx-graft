// Turns what a verify function returned into a Result: its status and, on
// OK, the result's JSON {"ok", "failOn", "counts", "error", "diagnostics"}.

package io.github.kengos.xlsxgraft;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;

final class VerifyReport {

  private VerifyReport() {}

  /**
   * The result for a verify function's {@code status} and, when the status is OK, its {@code json}.
   * It is a success when the result's {@code ok} is true, with no value; {@code counts} and {@code
   * failOn} are not kept (the counts are those of the diagnostics by severity).
   *
   * @throws XlsxGraftException the status is {@code XLSX_GRAFT_ERR_INVALID_ARG}, which only a call
   *     this package built wrongly gets
   */
  static Result<Void> call(int status, byte[] json) {
    if (status == StatusDiagnostic.OK) {
      return parsed(json);
    }
    if (status == StatusDiagnostic.INVALID_ARG) {
      throw new XlsxGraftException(
          "the engine refused a call this package built (status "
              + status
              + "): a bug of the package");
    }
    return Result.failure(Collections.<Diagnostic>emptyList(), StatusDiagnostic.of(status));
  }

  private static Result<Void> parsed(byte[] json) {
    try {
      Map<String, Object> report = EngineDiagnostics.object(Json.parse(json), "the result");
      List<Diagnostic> diagnostics = new ArrayList<>();
      for (Object record :
          EngineDiagnostics.of(
              List.class, EngineDiagnostics.member(report, "diagnostics"), "diagnostics")) {
        diagnostics.add(
            EngineDiagnostics.diagnostic(EngineDiagnostics.object(record, "a diagnostic")));
      }
      boolean ok =
          EngineDiagnostics.of(Boolean.class, EngineDiagnostics.member(report, "ok"), "ok");
      if (ok) {
        return Result.ok(null, diagnostics);
      }
      // `error` is read only when `ok` is false: a result that says ok is a success.
      return Result.failure(diagnostics, failure(EngineDiagnostics.member(report, "error")));
    } catch (Json.UnreadableException unreadable) {
      return Result.failure(
          Collections.<Diagnostic>emptyList(),
          StatusDiagnostic.error(
              "diagnostics_unreadable",
              "the engine's result could not be read: " + unreadable.getMessage()));
    }
  }

  // The result's `error` (io), or null when it has none.
  private static Diagnostic failure(Object error) throws Json.UnreadableException {
    if (error == null) {
      return null;
    }
    Map<String, Object> failure = EngineDiagnostics.object(error, "error");
    return StatusDiagnostic.error(
        EngineDiagnostics.text(failure, "code"), EngineDiagnostics.text(failure, "message"));
  }
}
