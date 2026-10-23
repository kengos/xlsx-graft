package io.github.kengos.xlsxgraft;

import java.util.Collections;
import java.util.List;

/**
 * What every call into the engine returns: whether it worked, the value when it did, and the
 * diagnostics either way.
 *
 * @param <T> what the call produces
 */
public final class Result<T> {

  private final boolean success;
  private final T value;
  private final List<Diagnostic> diagnostics;

  private Result(boolean success, T value, List<Diagnostic> diagnostics) {
    this.success = success;
    this.value = value;
    this.diagnostics = diagnostics;
  }

  static <T> Result<T> ok(T value) {
    return new Result<>(true, value, Collections.<Diagnostic>emptyList());
  }

  static <T> Result<T> failure(Diagnostic diagnostic) {
    return new Result<>(false, null, Collections.singletonList(diagnostic));
  }

  /**
   * Whether the call worked.
   *
   * @return true on a success
   */
  public boolean success() {
    return success;
  }

  /**
   * Whether it did not.
   *
   * @return true on a failure
   */
  public boolean failed() {
    return !success;
  }

  /**
   * What the call produced.
   *
   * @return the value on a success, null on a failure
   */
  public T value() {
    return value;
  }

  /**
   * What the engine reported, on a success as well as on a failure.
   *
   * @return the diagnostics, which cannot be changed
   */
  public List<Diagnostic> diagnostics() {
    return diagnostics;
  }
}
