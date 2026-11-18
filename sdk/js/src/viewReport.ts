/**
 * Turns what the addon's view functions returned into a {@link Result}: the
 * outcome's status and, for `XLSX_GRAFT_OK`, the view's JSON
 * `{"ok", "error", "diagnostics", ...}` followed by the keys of what the view
 * shows.
 */

import type { Outcome } from './addon.js';
import * as engineDiagnostics from './engineDiagnostics.js';
import { XlsxGraftError } from './errors.js';
import { Result } from './result.js';
import * as statusDiagnostic from './statusDiagnostic.js';

const NO_REASON = `the engine refused a value of the options (status ${statusDiagnostic.REQUEST_INVALID}) and gave no reason`;

/**
 * A successful result when the engine's `ok` is true. It holds the value
 * whether `ok` is true or not; the value is `null` only when the package
 * failed to get one (a status other than `XLSX_GRAFT_OK`, or JSON it cannot
 * read: a failure whose `error` is `diagnostics_unreadable`). `content` reads
 * the keys of what the view shows into its value: `viewContent.tree` or
 * `viewContent.grid`.
 *
 * Throws a RangeError, with the engine's message, for
 * `XLSX_GRAFT_ERR_REQUEST_INVALID`, and an {@link XlsxGraftError} for
 * `XLSX_GRAFT_ERR_INVALID_ARG`, which only a call this package built wrongly
 * gets.
 */
export function call<T>(
  outcome: Outcome,
  content: (view: Record<string, unknown>) => T,
): Result<T> {
  if (outcome.status === statusDiagnostic.OK) {
    // The addon sets `value` whenever the status is OK (engine/napi/src/call.rs).
    return parsed(outcome.value as string, content);
  }
  if (outcome.status === statusDiagnostic.REQUEST_INVALID) {
    throw new RangeError(outcome.message || NO_REASON);
  }
  if (outcome.status === statusDiagnostic.INVALID_ARG) {
    throw new XlsxGraftError(
      `the engine refused a call this package built (status ${outcome.status}): a bug of the package`,
    );
  }
  return Result.failure([], statusDiagnostic.call(outcome.status));
}

function parsed<T>(json: string, content: (view: Record<string, unknown>) => T): Result<T> {
  try {
    // Every object and array of the result is frozen, as a diagnostic's args or fix may nest.
    const view = engineDiagnostics.object(JSON.parse(json, (_key, value) => Object.freeze(value)));
    const value = content(view);
    const records = engineDiagnostics.field(view, 'diagnostics') as unknown[];
    const diagnostics = records.map((record) => engineDiagnostics.diagnostic(record));
    if (engineDiagnostics.field(view, 'ok')) {
      return Result.ok(value, diagnostics);
    }
    const error = engineDiagnostics.failure(engineDiagnostics.field(view, 'error'));
    return Result.failure(diagnostics, error, value);
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
