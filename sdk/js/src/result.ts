/** What every call into the engine returns. */

import type { Diagnostic } from './diagnostic.js';

/**
 * Whether the call worked, the value when it did, and the diagnostics either
 * way. `value` is `null` on a failure.
 */
export class Result<T> {
  readonly success: boolean;
  readonly value: T | null;
  readonly diagnostics: readonly Diagnostic[];

  constructor(success: boolean, value: T | null, diagnostics: readonly Diagnostic[] = []) {
    this.success = success;
    this.value = value;
    this.diagnostics = Object.freeze([...diagnostics]);
    Object.freeze(this);
  }

  static ok<T>(value: T, diagnostics: readonly Diagnostic[] = []): Result<T> {
    return new Result<T>(true, value, diagnostics);
  }

  static failure<T>(diagnostics: readonly Diagnostic[]): Result<T> {
    return new Result<T>(false, null, diagnostics);
  }

  get failed(): boolean {
    return !this.success;
  }
}
