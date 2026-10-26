/**
 * Turns a C ABI status other than `XLSX_GRAFT_OK` into a {@link Diagnostic}.
 *
 * The codes are provisional: they name the status codes of
 * engine/capi/include/xlsx_graft.h until the engine reports diagnostics of its
 * own, which replace them.
 */

import { Diagnostic } from './diagnostic.js';

export const OK = 0;

const CODES: ReadonlyMap<number, readonly [string, string]> = new Map([
  [1, ['null_arg', 'the engine was passed a null pointer']],
  [2, ['panic', 'the engine panicked; the call did not complete']],
]);

/** The diagnostic for `status`, the status a C ABI function returned. */
export function call(status: number): Diagnostic {
  const unknown = `the engine returned status ${status}, which this package does not know`;
  const [code, message] = CODES.get(status) ?? ['unknown_status', unknown];
  return new Diagnostic(code, message);
}
