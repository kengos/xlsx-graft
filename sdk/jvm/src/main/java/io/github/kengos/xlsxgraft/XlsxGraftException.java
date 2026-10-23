// What this package throws.
//
// A call into the engine never throws; it returns a Result. What throws is a
// setup that cannot reach the engine.

package io.github.kengos.xlsxgraft;

/** The root of what this package throws. */
public class XlsxGraftException extends RuntimeException {

  private static final long serialVersionUID = 1L;

  /**
   * Creates the exception with {@code message}.
   *
   * @param message what went wrong
   */
  public XlsxGraftException(String message) {
    super(message);
  }

  /**
   * Creates the exception with {@code message} over {@code cause}.
   *
   * @param message what went wrong
   * @param cause what it went wrong with
   */
  public XlsxGraftException(String message, Throwable cause) {
    super(message, cause);
  }
}
