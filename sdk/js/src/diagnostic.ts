/** One thing the engine reported about a call. */
export class Diagnostic {
  /** The stable, machine-readable part. */
  readonly code: string;
  /** For people. */
  readonly message: string;

  constructor(code: string, message: string) {
    this.code = code;
    this.message = message;
    Object.freeze(this);
  }
}
