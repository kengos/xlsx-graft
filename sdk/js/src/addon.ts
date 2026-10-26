/**
 * Finds and loads the engine's Node.js addon (`xlsx_graft.node`, built from
 * engine/napi). Node.js has no FFI on the lines this package supports, so it
 * loads an addon that links the C ABI in rather than the C ABI's library.
 *
 * Looked up in this order, and the first one set wins (a set path that does
 * not load throws rather than falls through):
 *
 * 1. the `XLSX_GRAFT_ADDON` environment variable
 * 2. the `path` option
 * 3. the copy shipped inside the package, at `native/xlsx_graft.node`
 *
 * The environment comes first because where the engine lives is the
 * deployment's decision, and it has to be able to override application code.
 * Nothing here downloads anything.
 */

import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { LibraryNotFoundError } from './errors.js';

export const ENV_VAR = 'XLSX_GRAFT_ADDON';

/** Where the package's own copy goes, beside dist/. */
export const PACKAGED = fileURLToPath(new URL('../native/xlsx_graft.node', import.meta.url));

/**
 * One call's result, as the addon returns it (`JsOutcome` in
 * engine/napi/src/shim.rs). `value` is set only when `status` is
 * `XLSX_GRAFT_OK`; otherwise the key is absent, as napi-rs leaves an object's
 * `None` field unset.
 */
export interface Outcome {
  status: number;
  value?: string;
}

/** The addon's exports, as this package calls them. Written by hand. */
export interface Addon {
  version(): Outcome;
}

/** One loaded engine addon. */
export interface Loaded {
  path: string;
  addon: Addon;
}

export interface OpenOptions {
  path?: string | undefined;
  env?: Readonly<Record<string, string | undefined>>;
  packaged?: string;
}

/**
 * The addon `env` or `path` names, or the packaged copy.
 *
 * Throws {@link LibraryNotFoundError} when no addon is found, it cannot be
 * loaded, or it does not export what this package calls.
 */
export function open({ path, env = process.env, packaged = PACKAGED }: OpenOptions = {}): Loaded {
  // An empty value counts as unset, as it does for the other SDKs.
  const found = env[ENV_VAR] || path || (existsSync(packaged) ? packaged : null);
  if (!found) {
    throw new LibraryNotFoundError(installHint('no engine addon was found', packaged));
  }

  const addon = load(found, packaged);
  if (typeof addon?.version !== 'function') {
    throw new LibraryNotFoundError(`${found} exports no \`version\``);
  }
  return { path: found, addon };
}

// Resolved against the working directory, as a path a user types is; the
// require() is anchored at this module and would resolve a relative path
// against it. Never unloaded: Node.js cannot unload an addon.
function load(found: string, packaged: string): Addon {
  try {
    return createRequire(import.meta.url)(resolve(found)) as Addon;
  } catch (error) {
    throw new LibraryNotFoundError(
      installHint(`${found} could not be loaded (${error})`, packaged),
    );
  }
}

function installHint(reason: string, packaged: string): string {
  return (
    `${reason}.\n\n` +
    'This package never downloads the engine. Point it at one of these ways:\n' +
    `  * set ${ENV_VAR} to the path of an xlsx_graft.node addon\n` +
    '  * pass version({ addon: "/path/to/xlsx_graft.node" })\n' +
    `  * put the addon at ${packaged}`
  );
}
