package io.github.kengos.xlsxgraft;

/** One thing the engine reported about a call. Two are equal when their parts are. */
public final class Diagnostic {

  private final String code;
  private final String message;

  Diagnostic(String code, String message) {
    this.code = code;
    this.message = message;
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
   * The part for people.
   *
   * @return the message
   */
  public String message() {
    return message;
  }

  @Override
  public boolean equals(Object other) {
    if (!(other instanceof Diagnostic)) {
      return false;
    }
    Diagnostic that = (Diagnostic) other;
    return code.equals(that.code) && message.equals(that.message);
  }

  @Override
  public int hashCode() {
    return 31 * code.hashCode() + message.hashCode();
  }

  @Override
  public String toString() {
    return code + ": " + message;
  }
}
