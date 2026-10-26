/** Calls the addon's `version` and returns the engine's version as a {@link Result}. */

import type { Addon } from './addon.js';
import { Result } from './result.js';
import * as statusDiagnostic from './statusDiagnostic.js';

/** The version string, such as `"0.0.0"`, on success. */
export function call(addon: Addon): Result<string> {
  const outcome = addon.version();
  if (outcome.status !== statusDiagnostic.OK) {
    return Result.failure([statusDiagnostic.call(outcome.status)]);
  }

  // The addon sets `value` whenever the status is OK (engine/napi/src/call.rs).
  return Result.ok(outcome.value as string);
}
