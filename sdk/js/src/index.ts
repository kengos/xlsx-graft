/**
 * Node.js bindings for xlsx-graft, an engine that reads, verifies, creates and
 * edits xlsx workbooks.
 *
 * The engine is a Node.js addon that links its C ABI in; nothing here
 * reimplements it. Calls return a {@link Result} rather than throwing. What
 * throws is a setup with no engine addon in it ({@link LibraryNotFoundError}).
 *
 * @example
 * ```ts
 * const result = version();
 * if (result.success) {
 *   console.log(result.value);
 * }
 * ```
 *
 * @module
 */

import { open } from './addon.js';
import type { Result } from './result.js';
import * as versionCall from './versionCall.js';

export { Diagnostic } from './diagnostic.js';
export { LibraryNotFoundError, XlsxGraftError } from './errors.js';
export { Result } from './result.js';

export interface VersionOptions {
  /** The engine addon's path, used when `XLSX_GRAFT_ADDON` is not set. */
  addon?: string;
}

/**
 * The engine's version, such as `"0.0.0"`, on success.
 *
 * Throws {@link LibraryNotFoundError} when no engine addon is found.
 */
export function version(options: VersionOptions = {}): Result<string> {
  return versionCall.call(open({ path: options.addon }).addon);
}
