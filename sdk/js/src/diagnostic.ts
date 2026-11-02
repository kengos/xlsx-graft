/**
 * One thing the engine reported about a call.
 *
 * The engine's own diagnostics (a read's `Result.diagnostics`) carry every
 * field of the JSON every host prints, keys as written there. One this
 * package makes from a C ABI status is an `error` with `args` and `location`
 * empty and no `decidedBy` or `fix`.
 */
export class Diagnostic {
  /** The stable, machine-readable part. */
  readonly code: string;
  /** `error`, `warning` or `notice`. */
  readonly severity: string;
  /** For people. */
  readonly message: string;
  /** The values the message names, by name. */
  readonly args: Readonly<Record<string, unknown>>;
  /** Where in the workbook: its part, cell or row, as the engine says. */
  readonly location: Readonly<Record<string, unknown>>;
  /** The rule or the source the engine followed, when it says. */
  readonly decidedBy: Readonly<Record<string, unknown>> | null;
  /** How to resolve it, when the caller can. */
  readonly fix: Readonly<Record<string, unknown>> | null;

  constructor({
    code,
    severity,
    message,
    args = {},
    location = {},
    decidedBy = null,
    fix = null,
  }: DiagnosticFields) {
    this.code = code;
    this.severity = severity;
    this.message = message;
    this.args = Object.freeze({ ...args });
    this.location = Object.freeze({ ...location });
    this.decidedBy = decidedBy === null ? null : Object.freeze({ ...decidedBy });
    this.fix = fix === null ? null : Object.freeze({ ...fix });
    Object.freeze(this);
  }
}

/** What a {@link Diagnostic} is made of; the last four may be left out. */
export interface DiagnosticFields {
  code: string;
  severity: string;
  message: string;
  args?: Record<string, unknown>;
  location?: Record<string, unknown>;
  decidedBy?: Record<string, unknown> | null;
  fix?: Record<string, unknown> | null;
}
