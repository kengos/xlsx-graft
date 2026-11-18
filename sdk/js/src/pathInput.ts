/**
 * Gives a path to the engine addon as a name, or as the bytes of the file when
 * the name cannot be given. Not exported from the package.
 */

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { Result } from './result.js';
import * as statusDiagnostic from './statusDiagnostic.js';

/**
 * Calls `byPath` with the name of `path` (a path or a `file:` URL) when the
 * name is well-formed UTF-16. For a name that is not (a lone surrogate) the
 * addon cannot be given the name, so this reads the file and calls `byBytes`
 * with its bytes: Node.js opens the name with U+FFFD in place of the lone
 * surrogate. A file that cannot be read is an `io` failure, and neither
 * function is called.
 *
 * Throws a TypeError for a URL that is not a `file:` URL.
 */
export function call<T>(
  path: string | URL,
  byPath: (name: string) => Result<T>,
  byBytes: (data: Buffer) => Result<T>,
): Result<T> {
  const name = typeof path === 'string' ? path : fileURLToPath(path);
  if (name.isWellFormed()) {
    return byPath(name);
  }
  let data: Buffer;
  try {
    data = readFileSync(name);
  } catch (error) {
    return failedToRead(error);
  }
  return byBytes(data);
}

function failedToRead<T>(error: unknown): Result<T> {
  return Result.failure(
    [],
    statusDiagnostic.of('io', `the file could not be read: ${(error as Error).message}`),
  );
}
