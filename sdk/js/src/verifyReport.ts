/**
 * Turns what the addon's verify functions returned into a {@link Result}: the
 * outcome's status and, for `XLSX_GRAFT_OK`, the result's JSON
 * `{"ok", "failOn", "counts", "error", "diagnostics"}`.
 */

import type { Outcome } from './addon.js';
import type { Diagnostic } from './diagnostic.js';
import * as engineDiagnostics from './engineDiagnostics.js';
import { XlsxGraftError } from './errors.js';
import { Result } from './result.js';
import * as statusDiagnostic from './statusDiagnostic.js';

/**
 * A successful result with no value when the engine's `ok` is true. `counts`
 * and `failOn` are not kept: the counts are those of `diagnostics` by
 * severity. A status other than `XLSX_GRAFT_OK` is a failure whose `error`
 * names it, and a result JSON that is not of the documented shape is a failure
 * whose `error` is `diagnostics_unreadable`.
 *
 * Throws an {@link XlsxGraftError} for `XLSX_GRAFT_ERR_INVALID_ARG`, which only
 * a call this package built wrongly gets.
 */
export function call(outcome: Outcome): Result<null> {
  if (outcome.status === statusDiagnostic.OK) {
    // The addon sets `value` whenever the status is OK (engine/napi/src/call.rs).
    return parsed(outcome.value as string);
  }
  if (outcome.status === statusDiagnostic.INVALID_ARG) {
    throw new XlsxGraftError(
      `the engine refused a call this package built (status ${outcome.status}): a bug of the package`,
    );
  }
  return Result.failure([], statusDiagnostic.call(outcome.status));
}

// The `error` of a result that is not ok: null, or an object with a code and a
// message.
function failure(error: unknown): Diagnostic | null {
  if (error === null) {
    return null;
  }
  const record = engineDiagnostics.object(error);
  return statusDiagnostic.of(
    engineDiagnostics.text(record, 'code'),
    engineDiagnostics.text(record, 'message'),
  );
}

function parsed(json: string): Result<null> {
  try {
    // Every object and array of the result is frozen, as a diagnostic's args or fix may nest.
    const report = engineDiagnostics.object(
      JSON.parse(json, (_key, value) => Object.freeze(value)),
    );
    const records = engineDiagnostics.field(report, 'diagnostics');
    const ok = engineDiagnostics.field(report, 'ok');
    if (!Array.isArray(records) || typeof ok !== 'boolean') {
      throw new TypeError('`diagnostics` is not an array or `ok` is not a boolean');
    }
    const diagnostics = records.map((record) => engineDiagnostics.diagnostic(record));
    if (ok) {
      return Result.ok(null, diagnostics);
    }
    return Result.failure(diagnostics, failure(engineDiagnostics.field(report, 'error')));
  } catch (error) {
    return Result.failure(
      [],
      statusDiagnostic.of(
        'diagnostics_unreadable',
        `the engine's result could not be read: ${(error as Error).message}`,
      ),
    );
  }
}
