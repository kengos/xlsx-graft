// Turns what a view function returned into a Result: its status and, on OK, the view's JSON
// {"ok", "error", "diagnostics", ...} with the keys of what the view shows, which the caller's
// Content reads into a Tree or a Grid. The diagnostics are read as the book's are
// (EngineDiagnostics.diagnostics).

package io.github.kengos.xlsxgraft;

import java.util.Collections;
import java.util.List;
import java.util.Map;

final class ViewReport {

  private static final String NO_REASON =
      "the engine refused a value of the options (status 11) and gave no reason";

  private ViewReport() {}

  /** Reads the keys of what a view shows into its value. */
  interface Content<T> {

    /**
     * The value {@code view} holds.
     *
     * @throws Json.UnreadableException a key is missing or of another type
     */
    T read(Map<String, Object> view) throws Json.UnreadableException;
  }

  /**
   * The result for a view function's {@code outcome}. It is a success when the view's {@code ok} is
   * true; else a failure with the view's diagnostics and its {@code error} as {@link
   * Result#error()}. The value is there either way; it is null only when the package got none (a
   * status other than OK, or JSON it cannot read).
   *
   * @throws IllegalArgumentException the engine refused a value of the options (REQUEST_INVALID),
   *     with its message
   * @throws XlsxGraftException the status is {@code XLSX_GRAFT_ERR_INVALID_ARG}, which only a call
   *     this package built wrongly gets
   */
  static <T> Result<T> call(ViewCall.Outcome outcome, Content<T> content) {
    if (outcome.status == StatusDiagnostic.OK) {
      return parsed(outcome.json, content);
    }
    if (outcome.status == StatusDiagnostic.REQUEST_INVALID) {
      throw new IllegalArgumentException(
          outcome.message == null || outcome.message.isEmpty() ? NO_REASON : outcome.message);
    }
    if (outcome.status == StatusDiagnostic.INVALID_ARG) {
      throw new XlsxGraftException(
          "the engine refused a call this package built (status "
              + outcome.status
              + "): a bug of the package");
    }
    return Result.failure(
        Collections.<Diagnostic>emptyList(), StatusDiagnostic.of(outcome.status), null);
  }

  private static <T> Result<T> parsed(byte[] json, Content<T> content) {
    try {
      Map<String, Object> view = EngineDiagnostics.object(Json.parse(json), "the result");
      T value = content.read(view);
      List<Diagnostic> diagnostics = EngineDiagnostics.diagnostics(view);
      boolean ok = EngineDiagnostics.of(Boolean.class, EngineDiagnostics.member(view, "ok"), "ok");
      if (ok) {
        return Result.ok(value, diagnostics);
      }
      // `error` is read only when `ok` is false: a view that says ok is a success.
      return Result.failure(
          diagnostics, EngineDiagnostics.failure(EngineDiagnostics.member(view, "error")), value);
    } catch (Json.UnreadableException unreadable) {
      return Result.failure(
          Collections.<Diagnostic>emptyList(),
          StatusDiagnostic.error(
              "diagnostics_unreadable",
              "the engine's result could not be read: " + unreadable.getMessage()),
          null);
    }
  }
}
