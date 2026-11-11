/**
 * Reads the JSON the addon's `book.diagnostics()` gives,
 * `{"has_errors": B, "error": E, "diagnostics": [...]}`.
 */

import { Diagnostic } from './diagnostic.js';
import * as statusDiagnostic from './statusDiagnostic.js';

/**
 * What the book said: whether a diagnostic is an error, its first failure,
 * and its diagnostics.
 */
export interface Report {
  hasErrors: boolean;
  error: Diagnostic | null;
  diagnostics: readonly Diagnostic[];
}

/** What a book whose diagnostics could not be read reports: nothing. */
export const UNREAD: Report = Object.freeze({ hasErrors: false, error: null, diagnostics: [] });

/** Throws a SyntaxError or a TypeError for JSON not of that shape. */
export function call(json: string): Report {
  // Frozen through, as a diagnostic's args or fix may nest.
  const report = object(JSON.parse(json, (_key, value) => Object.freeze(value)));
  const error = field(report, 'error');
  const diagnostics = field(report, 'diagnostics');
  if (!Array.isArray(diagnostics)) {
    throw new TypeError('`diagnostics` is not an array');
  }
  return {
    hasErrors: field(report, 'has_errors') === true,
    error:
      error === null
        ? null
        : statusDiagnostic.of(text(object(error), 'code'), text(object(error), 'message')),
    diagnostics: diagnostics.map((record) => diagnostic(record)),
  };
}

/**
 * A {@link Diagnostic} from one record of a `diagnostics` array, which every
 * host prints in the same form. Throws a TypeError for a value that is not
 * one.
 */
export function diagnostic(value: unknown): Diagnostic {
  const record = object(value);
  return new Diagnostic({
    code: text(record, 'code'),
    severity: text(record, 'severity'),
    message: text(record, 'message'),
    args: object(field(record, 'args')),
    location: object(field(record, 'location')),
    decidedBy: nullable(field(record, 'decidedBy')),
    fix: nullable(field(record, 'fix')),
  });
}

/** The value of `key`, which `record` must have. Throws a TypeError otherwise. */
export function field(record: Record<string, unknown>, key: string): unknown {
  if (!Object.hasOwn(record, key)) {
    throw new TypeError(`no \`${key}\``);
  }
  return record[key];
}

/** The string at `key`. Throws a TypeError for a value that is not a string. */
export function text(record: Record<string, unknown>, key: string): string {
  const value = field(record, key);
  if (typeof value !== 'string') {
    throw new TypeError(`\`${key}\` is not a string`);
  }
  return value;
}

/** `value` as an object. Throws a TypeError for an array, null or a scalar. */
export function object(value: unknown): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new TypeError(`${JSON.stringify(value)} is not an object`);
  }
  return value as Record<string, unknown>;
}

function nullable(value: unknown): Record<string, unknown> | null {
  return value === null ? null : object(value);
}
