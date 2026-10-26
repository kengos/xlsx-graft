/**
 * What this package throws.
 *
 * A call into the engine never throws; it returns a `Result`. What throws is a
 * setup that cannot reach the engine.
 */

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
