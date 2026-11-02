/**
 * Node.js bindings for xlsx-graft, an engine that reads, verifies, creates and
 * edits xlsx workbooks.
 *
 * The engine is a Node.js addon that links its C ABI in; nothing here
 * reimplements it. Calls return a {@link Result} rather than throwing. What
 * throws is a setup with no engine addon in it ({@link LibraryNotFoundError})
 * and a mistake in the caller's code, such as a book used after its callback
 * ({@link ClosedError}).
 *
 * @example
 * ```ts
 * const result = XlsxReader.open('book.xlsx', (book) => {
 *   for (const row of book.sheet().rows()) {
 *     console.log(row.number, row.values);
 *   }
 * });
 * if (result.error) {
 *   console.error(result.error.message);
 * }
 * ```
 *
 * @module
 */

import { open } from './addon.js';
import type { Result } from './result.js';
import * as versionCall from './versionCall.js';

export { Book, Sheet } from './book.js';
export { Diagnostic, type DiagnosticFields } from './diagnostic.js';
export { ClosedError, LibraryNotFoundError, XlsxGraftError } from './errors.js';
export { Result } from './result.js';
export {
  CalendarDate,
  CellError,
  Row,
  SheetInfo,
  type SheetKind,
  type SheetState,
  TimeOfDay,
  type Value,
} from './values.js';
export { type Read, type ReaderOptions, XlsxReader } from './xlsxReader.js';

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
