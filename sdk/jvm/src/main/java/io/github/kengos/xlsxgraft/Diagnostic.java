package io.github.kengos.xlsxgraft;

import java.util.Collections;
import java.util.Map;
import java.util.Objects;

/**
 * One thing the engine reported about a call.
 *
 * <p>The engine's own diagnostics ({@link Result#diagnostics()} of a read) carry every field of the
 * JSON every host prints. One the package makes from a C ABI status is an {@code "error"} with
 * {@link #args()} and {@link #location()} empty and no {@link #decidedBy()} or {@link #fix()}.
 *
 * <p>A JSON value is a {@code Map<String, Object>} (an object, in its order), a {@code
 * List<Object>}, a {@link String}, a {@link Long} (a whole number a {@code long} holds), a {@link
 * Double} (any other number), a {@link Boolean} or null; none can be changed. Two diagnostics are
 * equal when every field is.
 */
public final class Diagnostic {

  private final String code;
  private final String severity;
  private final String message;
  private final Map<String, Object> args;
  private final Map<String, Object> location;
  private final Map<String, Object> decidedBy;
  private final Object fix;

  /** One the package makes: no args, location, rule or fix. */
  Diagnostic(String code, String severity, String message) {
    this(
        code,
        severity,
        message,
        Collections.<String, Object>emptyMap(),
        Collections.<String, Object>emptyMap(),
        null,
        null);
  }

  Diagnostic(
      String code,
      String severity,
      String message,
      Map<String, Object> args,
      Map<String, Object> location,
      Map<String, Object> decidedBy,
      Object fix) {
    this.code = code;
    this.severity = severity;
    this.message = message;
    this.args = args;
    this.location = location;
    this.decidedBy = decidedBy;
    this.fix = fix;
  }

  /**
   * The stable, machine-readable part.
   *
   * @return the code, such as {@code "panic"}
   */
  public String code() {
    return code;
  }

  /**
   * How much it matters.
   *
   * @return {@code "error"}, {@code "warn"} or {@code "notice"}
   */
  public String severity() {
    return severity;
  }

  /**
   * The part for people.
   *
   * @return the message
   */
  public String message() {
    return message;
  }

  /**
   * The values the message was made from.
   *
   * @return a JSON object, empty for none
   */
  public Map<String, Object> args() {
    return args;
  }

  /**
   * Where in the workbook.
   *
   * @return a JSON object (its {@code part}, cell, ...), empty for none
   */
  public Map<String, Object> location() {
    return location;
  }

  /**
   * The rule that decided it (the JSON's {@code decidedBy}).
   *
   * @return a JSON object, or null when the engine names none
   */
  public Map<String, Object> decidedBy() {
    return decidedBy;
  }

  /**
   * How to fix it.
   *
   * @return a JSON value, or null when the engine says not
   */
  public Object fix() {
    return fix;
  }

  @Override
  public boolean equals(Object other) {
    if (!(other instanceof Diagnostic)) {
      return false;
    }
    Diagnostic that = (Diagnostic) other;
    return code.equals(that.code)
        && severity.equals(that.severity)
        && message.equals(that.message)
        && args.equals(that.args)
        && location.equals(that.location)
        && Objects.equals(decidedBy, that.decidedBy)
        && Objects.equals(fix, that.fix);
  }

  @Override
  public int hashCode() {
    return Objects.hash(code, severity, message, args, location, decidedBy, fix);
  }

  @Override
  public String toString() {
    return code + ": " + message;
  }
}
