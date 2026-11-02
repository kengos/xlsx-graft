package io.github.kengos.xlsxgraft;

import java.util.Collections;
import java.util.List;

/**
 * What every call into the engine returns: whether it worked, the value when it did, why it did not
 * when it stopped, and the diagnostics either way. {@link Book#result()} is a {@code Result<Void>}:
 * a try-with-resources block has no value to give.
 *
 * <p>{@link #error()} says why the call stopped short: the workbook or a sheet asked for was
 * refused, or the host or the engine failed. A call can fail with no error too: a read that a
 * problem in its sheet stopped, whose diagnostics hold an error, as {@code xlsx-graft read}'s
 * {@code ok} is false then.
 *
 * @param <T> what the call produces
 */
public final class Result<T> {

  private final boolean success;
  private final T value;
  private final List<Diagnostic> diagnostics;
  private final Diagnostic error;

  private Result(boolean success, T value, List<Diagnostic> diagnostics, Diagnostic error) {
    this.success = success;
    this.value = value;
    this.diagnostics = Collections.unmodifiableList(diagnostics);
    this.error = error;
  }

  static <T> Result<T> ok(T value) {
    return ok(value, Collections.<Diagnostic>emptyList());
  }

  static <T> Result<T> ok(T value, List<Diagnostic> diagnostics) {
    return new Result<>(true, value, diagnostics, null);
  }

  static <T> Result<T> failure(Diagnostic diagnostic) {
    return new Result<>(false, null, Collections.singletonList(diagnostic), null);
  }

  static <T> Result<T> failure(List<Diagnostic> diagnostics, Diagnostic error) {
    return new Result<>(false, null, diagnostics, error);
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

  /**
   * The first thing that stopped the call short.
   *
   * @return the error, or null
   */
  public Diagnostic error() {
    return error;
  }
}
