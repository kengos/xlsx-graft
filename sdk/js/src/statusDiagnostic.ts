/**
 * Turns a C ABI status other than `XLSX_GRAFT_OK` into a {@link Diagnostic},
 * for a status the engine keeps no failure of: one with no book to keep it
 * (an open), or after which its diagnostics cannot be read (a panic).
 *
 * The statuses that refuse a sheet are named as `xlsx-graft read` names them
 * in its end line's `error.code`; the book's diagnostics say them in full,
 * naming the sheet. The others name the status codes of
 * engine/capi/include/xlsx_graft.h.
 */

import { Diagnostic } from './diagnostic.js';

export const OK = 0;
export const INVALID_ARG = 4;

const CODES: ReadonlyMap<number, readonly [string, string]> = new Map([
  [1, ['null_arg', 'the engine was passed a null pointer']],
  [2, ['panic', 'the engine panicked; the call did not complete']],
  [3, ['io', 'the host failed to open, read or write a file']],
  [INVALID_ARG, ['invalid_arg', 'the engine was passed text that is not UTF-8']],
  [5, ['workbook_unreadable', 'the workbook cannot be read; the diagnostics say why']],
  [6, ['sheet_not_found', 'no such sheet']],
  [7, ['sheet_ambiguous', 'more than one sheet has the name, letter case aside']],
  [8, ['sheet_not_worksheet', 'the sheet is not a worksheet']],
  [9, ['sheet_unreadable', 'the sheet cannot be read; the diagnostics say why']],
  [10, ['closed', "the rows' book was closed"]],
]);

/**
 * The diagnostic for `status`, the status a C ABI function returned;
 * `message` in place of the status's own, for one that knows more (why the
 * host failed).
 */
export function call(status: number, message?: string | null): Diagnostic {
  const unknown = `the engine returned status ${status}, which this package does not know`;
  const [code, own] = CODES.get(status) ?? ['unknown_status', unknown];
  return of(code, message ?? own);
}

/** An `error` diagnostic of this package's own, not from the engine's. */
export function of(code: string, message: string): Diagnostic {
  return new Diagnostic({ code, severity: 'error', message });
}
