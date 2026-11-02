// Reads the JSON xlsx_graft_book_diagnostics writes,
// {"has_errors": B, "error": E, "diagnostics": [...]}.

package io.github.kengos.xlsxgraft;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;

final class EngineDiagnostics {

  /** What a book whose diagnostics could not be read reports: nothing but the package's failure. */
  static final Report UNREAD = new Report(false, null, Collections.<Diagnostic>emptyList());

  private EngineDiagnostics() {}

  /**
   * The report {@code json} holds.
   *
   * @throws Json.UnreadableException the bytes are not JSON of that shape
   */
  static Report call(byte[] json) throws Json.UnreadableException {
    Map<String, Object> report = object(Json.parse(json), "the diagnostics");
    Object error = member(report, "error");
    List<Diagnostic> diagnostics = new ArrayList<>();
    for (Object record : of(List.class, member(report, "diagnostics"), "diagnostics")) {
      diagnostics.add(diagnostic(object(record, "a diagnostic")));
    }
    Map<String, Object> failure = error == null ? null : object(error, "error");
    return new Report(
        of(Boolean.class, member(report, "has_errors"), "has_errors"),
        failure == null
            ? null
            : StatusDiagnostic.error(text(failure, "code"), text(failure, "message")),
        diagnostics);
  }

  private static Diagnostic diagnostic(Map<String, Object> record) throws Json.UnreadableException {
    Object decidedBy = member(record, "decidedBy");
    return new Diagnostic(
        text(record, "code"),
        text(record, "severity"),
        text(record, "message"),
        object(member(record, "args"), "args"),
        object(member(record, "location"), "location"),
        decidedBy == null ? null : object(decidedBy, "decidedBy"),
        member(record, "fix"));
  }

  // A member that is there, null or not.
  private static Object member(Map<String, Object> record, String name)
      throws Json.UnreadableException {
    if (!record.containsKey(name)) {
      throw new Json.UnreadableException("no `" + name + "`");
    }
    return record.get(name);
  }

  private static String text(Map<String, Object> record, String name)
      throws Json.UnreadableException {
    return of(String.class, member(record, name), name);
  }

  @SuppressWarnings("unchecked") // Json reads every object as a Map<String, Object>.
  private static Map<String, Object> object(Object value, String name)
      throws Json.UnreadableException {
    return (Map<String, Object>) of(Map.class, value, name);
  }

  private static <T> T of(Class<T> type, Object value, String name)
      throws Json.UnreadableException {
    if (!type.isInstance(value)) {
      throw new Json.UnreadableException(
          "`"
              + name
              + "` is "
              + (value == null ? "null" : "a " + value.getClass().getSimpleName())
              + ", not a "
              + type.getSimpleName());
    }
    return type.cast(value);
  }

  /** What the book said. */
  static final class Report {

    /** Whether a diagnostic is an error. */
    final boolean hasErrors;

    /** Its first failure, or null. */
    final Diagnostic error;

    final List<Diagnostic> diagnostics;

    Report(boolean hasErrors, Diagnostic error, List<Diagnostic> diagnostics) {
      this.hasErrors = hasErrors;
      this.error = error;
      this.diagnostics = diagnostics;
    }
  }
}
