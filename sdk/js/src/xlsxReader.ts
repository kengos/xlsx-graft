/** Opens a workbook to read its sheets' rows, through the engine addon. */

import { fileURLToPath } from 'node:url';
import { type Addon, type AddonBook, open as openAddon } from './addon.js';
import { Book } from './book.js';
import { BookCore } from './bookCore.js';
import { LibraryNotFoundError } from './errors.js';
import { Result } from './result.js';
import * as statusDiagnostic from './statusDiagnostic.js';
import { Zone } from './zone.js';

export interface ReaderOptions {
  /**
   * The zone a date-time cell is read in, UTC unless given: any zone
   * `Intl.DateTimeFormat` takes, an IANA name (`Asia/Tokyo`) or an offset
   * (`+09:00`). The workbook's are wall-clock times with no zone.
   */
  timezone?: string;
  /** The engine addon's path, used when `XLSX_GRAFT_ADDON` is not set. */
  addon?: string;
}

/** What the callback is handed the book in. */
export type Read<T> = (book: Book) => T;

/**
 * Opens the workbook at `path`, hands it to `read` as a {@link Book}, and
 * closes it when `read` returns, however it returns. An exception `read`
 * throws is the caller's: it propagates, the book closed first. `read` must
 * be synchronous: the book is closed when it returns.
 *
 * The result's value is what `read` returned, on success. It fails with
 * `error` set when the file cannot be read (and `read` is not called), the
 * workbook or a sheet asked for is refused, or the engine fails; and
 * without, when a problem stopped a sheet's rows, its diagnostic an error.
 *
 * Throws a `LibraryNotFoundError` when no engine addon is found, a
 * RangeError for a zone `Intl` does not take, and a TypeError when `read` is
 * not a function or returns a promise.
 *
 * @example
 * ```ts
 * const result = XlsxReader.open('book.xlsx', { timezone: 'Asia/Tokyo' }, (book) => {
 *   for (const row of book.sheet('Data').rows()) {
 *     console.log(row.number, row.values);
 *   }
 * });
 * for (const d of result.diagnostics) console.warn(`${d.severity} ${d.code}: ${d.message}`);
 * if (result.error) console.warn(result.error.message);
 * ```
 */
function open<T>(path: string | URL, read: Read<T>): Result<T>;
function open<T>(path: string | URL, options: ReaderOptions | undefined, read: Read<T>): Result<T>;
function open<T>(
  path: string | URL,
  ...rest: [Read<T>] | [ReaderOptions | undefined, Read<T>]
): Result<T> {
  const [options, read] = split(rest);
  const name = typeof path === 'string' ? path : fileURLToPath(path);
  return call(options, read, (addon) => (name.isWellFormed() ? addon.Book.openPath(name) : null));
}

/** As {@link open}, the workbook being the bytes of `data`, which the engine copies. */
function openBytes<T>(data: Uint8Array, read: Read<T>): Result<T>;
function openBytes<T>(
  data: Uint8Array,
  options: ReaderOptions | undefined,
  read: Read<T>,
): Result<T>;
function openBytes<T>(
  data: Uint8Array,
  ...rest: [Read<T>] | [ReaderOptions | undefined, Read<T>]
): Result<T> {
  const [options, read] = split(rest);
  // A view of the same memory, as the addon takes a Buffer.
  const buffer = Buffer.from(data.buffer, data.byteOffset, data.byteLength);
  return call(options, read, (addon) => addon.Book.openBytes(buffer));
}

/** Reads a workbook: `XlsxReader.open(path, ...)` or `XlsxReader.openBytes(data, ...)`. */
export const XlsxReader = Object.freeze({ open, openBytes });

/** The options and the callback; options left out, or `undefined`, are none. */
function split<T>(
  rest: [Read<T>] | [ReaderOptions | undefined, Read<T>],
): [ReaderOptions, Read<T>] {
  return rest.length === 1 ? [{}, rest[0]] : [rest[0] ?? {}, rest[1]];
}

/** Opens the book `how` opens, from the addon `options` names, and reads it with `read`. */
function call<T>(
  { timezone = 'UTC', addon }: ReaderOptions,
  read: Read<T>,
  how: (addon: Addon) => AddonBook | null,
): Result<T> {
  if (typeof read !== 'function') {
    throw new TypeError('a callback is required: the book is closed when it returns');
  }
  const zone = new Zone(timezone);
  const loaded = openAddon({ path: addon }).addon;
  if (typeof loaded.Book?.openPath !== 'function' || typeof loaded.Book.openBytes !== 'function') {
    throw new LibraryNotFoundError('the engine addon exports no `Book`');
  }
  return reading(how(loaded), zone, read);
}

/**
 * Reads `book` with `read`; `null` is a path the engine was not called with,
 * as it is no UTF-8.
 */
export function reading<T>(book: AddonBook | null, zone: Zone, read: Read<T>): Result<T> {
  if (book === null) {
    return Result.failure([], statusDiagnostic.call(statusDiagnostic.INVALID_ARG));
  }
  if (book.status !== statusDiagnostic.OK) {
    return Result.failure([], statusDiagnostic.call(book.status, book.message));
  }
  const core = new BookCore(book, zone);
  let value: T;
  try {
    value = read(new Book(core));
  } finally {
    core.close();
  }
  if (typeof (value as { then?: unknown } | null)?.then === 'function') {
    // The TypeError is the caller's signal; what the promise does after,
    // using a closed book, is not left to crash the process unhandled.
    (value as PromiseLike<unknown>).then(undefined, () => {});
    throw new TypeError('the callback returned a promise: the book is closed when it returns');
  }
  return core.result(value);
}
