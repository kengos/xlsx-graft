package io.github.kengos.xlsxgraft;

/**
 * No engine library could be found or loaded, or it lacks a function this package calls.
 *
 * <p>A configuration mistake, not a failed call, so it is thrown rather than returned as a {@link
 * Result}.
 */
public class LibraryNotFoundException extends XlsxGraftException {

  private static final long serialVersionUID = 1L;

  /**
   * Creates the exception with {@code message}.
   *
   * @param message what went wrong
   */
  public LibraryNotFoundException(String message) {
    super(message);
  }

  /**
   * Creates the exception with {@code message} over {@code cause}.
   *
   * @param message what went wrong
   * @param cause what it went wrong with
   */
  public LibraryNotFoundException(String message, Throwable cause) {
    super(message, cause);
  }
}
