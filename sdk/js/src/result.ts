/** What every call into the engine returns. */

import type { Diagnostic } from './diagnostic.js';

/**
 * Whether the call worked, the value when it did, why it did not when it
 * stopped, and the diagnostics either way. `value` is `null` on a failure.
 *
 * `error` says why the call stopped short: the workbook or a sheet asked for
 * was refused, or the host or the engine failed. A call can fail with no
 * `error` too: a read that a problem in its sheet stopped, whose diagnostics
 * hold an error, as `xlsx-graft read`'s `ok` is false then.
 */
export class Result<T> {
  readonly success: boolean;
  readonly value: T | null;
  readonly error: Diagnostic | null;
  readonly diagnostics: readonly Diagnostic[];

  constructor(
    success: boolean,
    value: T | null,
    diagnostics: readonly Diagnostic[] = [],
    error: Diagnostic | null = null,
  ) {
    this.success = success;
    this.value = value;
    this.error = error;
    this.diagnostics = Object.freeze([...diagnostics]);
    Object.freeze(this);
  }

  static ok<T>(value: T, diagnostics: readonly Diagnostic[] = []): Result<T> {
    return new Result<T>(true, value, diagnostics);
  }

  static failure<T>(
    diagnostics: readonly Diagnostic[],
    error: Diagnostic | null = null,
  ): Result<T> {
    return new Result<T>(false, null, diagnostics, error);
  }

  get failed(): boolean {
    return !this.success;
  }
}
