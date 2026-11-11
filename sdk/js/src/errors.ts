/**
 * What this package throws.
 *
 * A call into the engine never throws; it returns a `Result`. What throws is a
 * setup that cannot reach the engine ({@link LibraryNotFoundError}), a policy
 * the engine does not take, given to a `Verifier` when it is made
 * ({@link PolicyInvalidError}), a book used after its callback
 * ({@link ClosedError}), and a call this package built wrongly, which the
 * engine refuses (an {@link XlsxGraftError}: a bug of the package). A call
 * made wrongly by the caller throws JavaScript's own errors: a RangeError for
 * a policy and rules of more than 2 MiB of JSON, and a TypeError for a value
 * JSON cannot write or a URL that is not a `file:` URL.
 */

import type { Diagnostic } from './diagnostic.js';

/** The root of what this package throws. */
export class XlsxGraftError extends Error {
  override name = 'XlsxGraftError';
}

/**
 * No engine addon could be found or loaded.
 *
 * A configuration mistake, not a failed call, so it is thrown rather than
 * returned as a `Result`.
 */
export class LibraryNotFoundError extends XlsxGraftError {
  override name = 'LibraryNotFoundError';
}

/**
 * A book used after its callback returned, when it is closed.
 *
 * A mistake in the caller's code, not a failed call, so it is thrown.
 */
export class ClosedError extends XlsxGraftError {
  override name = 'ClosedError';
}

/**
 * The engine does not take the policy given to a `Verifier`.
 *
 * A mistake in the caller's policy, not a failed call, so it is thrown. It is
 * thrown too when the check of the policy itself fails: then the first of
 * `diagnostics` is that failure's error, its code `panic`, `null_arg`,
 * `unknown_status` or `diagnostics_unreadable`.
 */
export class PolicyInvalidError extends XlsxGraftError {
  override name = 'PolicyInvalidError';
  /**
   * What is wrong, a `policy_invalid` diagnostic for each key, and the
   * engine's `error` first when it gave one.
   */
  readonly diagnostics: readonly Diagnostic[];

  constructor(diagnostics: readonly Diagnostic[]) {
    super(`the policy is not valid: ${diagnostics.map((d) => d.message).join('; ')}`);
    this.diagnostics = Object.freeze([...diagnostics]);
  }
}
