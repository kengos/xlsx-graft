package io.github.kengos.xlsxgraft;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Thrown by {@link Verifier} when the engine does not take the policy it is given: a key it does
 * not know, a limit of 0 or above its maximum, or a forbidden part of an unknown kind. It is also
 * thrown when the check of the policy itself fails; the first of {@link #diagnostics()} is then
 * that error, with the code {@code panic}, {@code null_arg}, {@code unknown_status} or {@code
 * diagnostics_unreadable}.
 */
public final class PolicyInvalidException extends XlsxGraftException {

  private static final long serialVersionUID = 1L;

  // Diagnostic is not Serializable, so a deserialized exception has no diagnostics.
  private final transient List<Diagnostic> diagnostics;

  /** The exception whose message joins the messages of {@code diagnostics}. */
  PolicyInvalidException(List<Diagnostic> diagnostics) {
    super("the policy is not valid: " + messages(diagnostics));
    this.diagnostics = Collections.unmodifiableList(new ArrayList<>(diagnostics));
  }

  /**
   * What the engine said was wrong with the policy: the error of the check when it failed, then one
   * {@code policy_invalid} diagnostic for each fault.
   *
   * @return the diagnostics, which cannot be changed; null for an exception read back from its
   *     serialized form
   */
  public List<Diagnostic> diagnostics() {
    return diagnostics;
  }

  private static String messages(List<Diagnostic> diagnostics) {
    StringBuilder joined = new StringBuilder();
    for (Diagnostic diagnostic : diagnostics) {
      if (joined.length() > 0) {
        joined.append("; ");
      }
      joined.append(diagnostic.message());
    }
    return joined.toString();
  }
}
