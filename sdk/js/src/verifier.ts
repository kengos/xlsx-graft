/**
 * Verifies workbooks through the engine addon, under the host's policy and the
 * rules of the caller.
 */

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { type Addon, open as openAddon } from './addon.js';
import { LibraryNotFoundError, PolicyInvalidError } from './errors.js';
import { Result } from './result.js';
import * as statusDiagnostic from './statusDiagnostic.js';
import * as verifyReport from './verifyReport.js';

// The most JSON the engine takes as the policy and the rules together.
const MAX_OPTIONS_BYTES = 2 << 20;

export interface VerifierOptions {
  /**
   * The host's policy: the engine's limits, `failOn`, `denyScheduled` and
   * `forbid` (`{ functions: [...], parts: [...] }`), written as the engine
   * names them (camelCase); no key is renamed and no value is changed. `null`
   * or left out is the engine's defaults. The policy is copied as JSON when
   * the verifier is made: changing `policy` after changes nothing. A lone
   * surrogate in a string of the policy (a key too) is written as U+FFFD, and
   * the engine checks that string as it checks any other.
   */
  policy?: Readonly<Record<string, unknown>> | null;
  /** The engine addon's path, used when `XLSX_GRAFT_ADDON` is not set. */
  addon?: string;
}

/**
 * The rules of one call, each optional; `null` is the same as left out. A
 * wrong value is not thrown for: the engine reports it as a `rules_invalid`
 * or `target_invalid` diagnostic and does not open the workbook.
 */
export interface VerifyRules {
  /** The versions the workbook must run in (`"excel >= 2016"`); a problem with one is an error. */
  targets?: readonly string[] | null;
  /** The versions the workbook should run in; a problem with one is a warning. */
  bestEffort?: readonly string[] | null;
  /** Functions the engine refuses by default that are allowed, named without an `_xlfn.` prefix. */
  allowFunctions?: readonly string[] | null;
  /**
   * Kinds of part the engine refuses by default that are allowed (`macro`,
   * `external_link`, `dde`, `ole`, `activex`, `data_connection`,
   * `web_publish`).
   */
  allowParts?: readonly string[] | null;
  /**
   * `"warn"` to fail on a warning too; the stricter of this and the policy's
   * `failOn` counts.
   */
  failOn?: string | null;
}

/**
 * Verifies workbooks under the host's policy (given to the constructor, which
 * has the engine check it) and the rules of the caller (given to each call).
 *
 * A verifier does not change once it is made, and a call keeps no state in
 * it. A worker thread makes its own verifier: an object is not shared between
 * threads.
 *
 * @example
 * ```ts
 * const verifier = new Verifier({ policy: { maxFormulaBytes: 32768 } });
 * const result = verifier.verify('book.xlsx', { targets: ['excel >= 2016'], allowParts: ['macro'] });
 * for (const d of result.diagnostics) console.error(`${d.severity} ${d.code}: ${d.message}`);
 * if (result.error) console.error(result.error.message);
 * ```
 */
export class Verifier {
  readonly #addon: Addon;
  // The policy's JSON as the engine accepted it; the string `null` when no policy was given.
  readonly #policy: string;

  /**
   * Throws a {@link PolicyInvalidError} when the engine does not take the
   * policy, a {@link LibraryNotFoundError} when no engine addon is found or
   * the addon of an older engine lacks the verify functions, and a TypeError
   * for a policy that holds a value JSON cannot write (a BigInt, a cycle).
   */
  constructor({ policy, addon }: VerifierOptions = {}) {
    const loaded = openAddon({ path: addon }).addon;
    if (
      typeof loaded.verifyPath !== 'function' ||
      typeof loaded.verifyBytes !== 'function' ||
      typeof loaded.checkPolicy !== 'function'
    ) {
      throw new LibraryNotFoundError(
        'the engine addon exports no `verifyPath`, `verifyBytes` or `checkPolicy`',
      );
    }
    this.#addon = loaded;
    this.#policy = policy == null ? 'null' : checked(loaded, JSON.stringify(policy, wellFormed()));
    Object.freeze(this);
  }

  /**
   * Verifies the workbook at `path`, a path or a `file:` URL.
   *
   * The result is successful when the engine's `ok` is true, and it has no
   * value (`value` is `null`). A file that cannot be read is an `io` `error`.
   * A fault of the rules is a `rules_invalid` or `target_invalid` diagnostic,
   * and the workbook is not opened. With `failOn: 'warn'` a result fails on a
   * warning with no error diagnostic. For a path that is not well-formed
   * UTF-16 (a lone surrogate) the engine cannot be given the name, so this
   * package reads the file and verifies its bytes: Node.js opens the name with
   * U+FFFD in place of the lone surrogate, and an unreadable file is an `io`
   * `error`, and the rules are not checked.
   *
   * A lone surrogate in a string of the rules is written as U+FFFD, and the
   * engine checks that string as it checks any other.
   *
   * Throws a RangeError for a policy and rules that make more than 2 MiB of
   * JSON, and a TypeError for a rule that JSON cannot write or for a URL that
   * is not a `file:` URL.
   */
  verify(path: string | URL, rules: VerifyRules = {}): Result<null> {
    const options = this.#options(rules);
    const name = typeof path === 'string' ? path : fileURLToPath(path);
    if (name.isWellFormed()) {
      return verifyReport.call(this.#addon.verifyPath(name, options));
    }
    let data: Buffer;
    try {
      data = readFileSync(name);
    } catch (error) {
      return failedToRead(error);
    }
    return verifyReport.call(this.#addon.verifyBytes(data, options));
  }

  /**
   * As {@link Verifier.verify}, the workbook being the bytes of `data`, read
   * in place for the call and not copied: JavaScript waits on the call, so
   * `data` does not change during it. A `SharedArrayBuffer` another thread
   * writes during the call is the caller's to prevent.
   *
   * Throws as {@link Verifier.verify} does.
   */
  verifyBytes(data: Uint8Array, rules: VerifyRules = {}): Result<null> {
    const options = this.#options(rules);
    // A view of the same memory, as the addon takes a Buffer.
    const buffer = Buffer.from(data.buffer, data.byteOffset, data.byteLength);
    return verifyReport.call(this.#addon.verifyBytes(buffer, options));
  }

  // The options JSON: the policy and the rules the caller gave, by the
  // engine's names.
  #options(rules: VerifyRules): string {
    const { targets, bestEffort, allowFunctions, allowParts, failOn } = rules;
    const wanted = {
      targets,
      bestEffort,
      functions: allowFunctions == null ? null : { allow: allowFunctions },
      parts: allowParts == null ? null : { allow: allowParts },
      failOn,
    };
    const given = Object.fromEntries(Object.entries(wanted).filter(([, value]) => value != null));
    const options = `{"policy":${this.#policy},"rules":${JSON.stringify(given, wellFormed())}}`;
    const size = Buffer.byteLength(options);
    if (size > MAX_OPTIONS_BYTES) {
      throw new RangeError(
        `the policy and the rules are ${size} bytes of JSON; the engine takes ${MAX_OPTIONS_BYTES}`,
      );
    }
    return options;
  }
}

// A JSON.stringify replacer: a lone surrogate in a string, a key included, is
// written as U+FFFD. Written as the escape JSON.stringify makes of it (`\ud800`),
// the engine's JSON parser refuses it, and a refusal of the options is the
// engine's INVALID_ARG, which is a bug of this package, not of the caller. As
// U+FFFD the string is checked by the engine as any other.
//
// An object whose keys are all well-formed is returned as it is. One with a
// lone surrogate in a key is replaced by a copy; the copy is made once for
// each object of one stringify (`copies`), so that an object that refers to
// itself is still found by JSON.stringify and throws its TypeError.
function wellFormed(): (key: string, value: unknown) => unknown {
  const copies = new WeakMap<object, object>();
  return (_key, value) => {
    if (typeof value === 'string') {
      return value.toWellFormed();
    }
    if (typeof value !== 'object' || value === null || Array.isArray(value)) {
      return value;
    }
    const entries = Object.entries(value);
    if (entries.every(([key]) => key.isWellFormed())) {
      return value;
    }
    let copy = copies.get(value);
    if (copy === undefined) {
      copy = Object.fromEntries(entries.map(([key, item]) => [key.toWellFormed(), item]));
      copies.set(value, copy);
    }
    return copy;
  };
}

// Returns `policy` when the engine accepts it; throws PolicyInvalidError otherwise.
function checked(addon: Addon, policy: string): string {
  const result = verifyReport.call(addon.checkPolicy(policy));
  if (result.failed) {
    throw new PolicyInvalidError([...(result.error ? [result.error] : []), ...result.diagnostics]);
  }
  return policy;
}

function failedToRead(error: unknown): Result<null> {
  return Result.failure(
    [],
    statusDiagnostic.of('io', `the file could not be read: ${(error as Error).message}`),
  );
}
