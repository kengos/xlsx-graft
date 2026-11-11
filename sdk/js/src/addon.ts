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
 * 3. the addon of this platform's package, `@xlsx-graft/<os>-<cpu>[-gnu|-msvc]`
 *    (see {@link platformPackage}), which npm installs beside this package as
 *    one of its optional dependencies when the platform fits
 *
 * The environment comes first because where the engine lives is the
 * deployment's decision, and it has to be able to override application code.
 * Nothing here downloads anything.
 */

import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { LibraryNotFoundError } from './errors.js';

export const ENV_VAR = 'XLSX_GRAFT_ADDON';

/**
 * The npm package that holds the addon of an operating system and processor,
 * or null for one no package is built for. Named as Node.js names them
 * (`process.platform`, `process.arch`), with the C library the Linux addon is
 * linked against and the toolchain of the Windows one, as the package's
 * `os`, `cpu` and `libc` fields select it.
 */
export function platformPackage(
  platform: string = process.platform,
  arch: string = process.arch,
): string | null {
  const cpu = arch === 'x64' || arch === 'arm64' ? arch : null;
  if (platform === 'linux' && cpu) return `@xlsx-graft/linux-${cpu}-gnu`;
  if (platform === 'darwin' && cpu) return `@xlsx-graft/darwin-${cpu}`;
  if (platform === 'win32' && cpu === 'x64') return '@xlsx-graft/win32-x64-msvc';
  return null;
}

const requireHere = createRequire(import.meta.url);

/**
 * The path of the addon in this platform's package, or null when the package
 * is not installed (npm skips an optional package that does not fit, and a
 * `--omit=optional` install leaves it out).
 */
export function packagedAddon(
  platformName: string | null = platformPackage(),
  resolveModule: (id: string) => string = (id) => requireHere.resolve(id),
): string | null {
  if (platformName === null) return null;
  try {
    return resolveModule(`${platformName}/xlsx_graft.node`);
  } catch {
    return null;
  }
}

/**
 * One call's result, as the addon returns it (`JsOutcome` in
 * engine/napi/src/shim.rs). `value` is set only when `status` is
 * `XLSX_GRAFT_OK`; otherwise the key is absent, as napi-rs leaves an object's
 * `None` field unset.
 */
export interface Outcome {
  status: number;
  value?: string;
  /** Why the host failed, set only when `status` is `XLSX_GRAFT_ERR_IO`. */
  message?: string;
}

/**
 * A workbook the addon opened (`Book` in engine/napi/src/shim.rs): `status`
 * first, and every handle call's status as the C ABI's header gives it.
 */
export interface AddonBook {
  readonly status: number;
  readonly message?: string | null;
  sheets(): { status: number; sheets: { name: string; kind: string; state: string }[] };
  rows(index: number): AddonRows;
  rowsNamed(name: string): AddonRows;
  diagnostics(): Outcome;
  close(): void;
}

/** The rows of one sheet (`Rows` in engine/napi/src/shim.rs). */
export interface AddonRows {
  readonly status: number;
  nextRow(): { status: number; message?: string; row?: { number: number; cells: AddonCell[] } };
  close(): void;
}

/**
 * One cell (`JsCell` in engine/napi/src/shim.rs): its type, as
 * `read --format json` names it or `unknown`, and the fields that type sets.
 */
export interface AddonCell {
  type: string;
  text?: string;
  number?: number;
  boolean?: boolean;
  date?: number[];
  time?: number[];
  milliseconds?: number;
}

/** The addon's exports, as this package calls them. Written by hand. */
export interface Addon {
  version(): Outcome;
  /**
   * Verifies the workbook at `path`; `options` is the JSON
   * `{"policy": ..., "rules": ...}`. The outcome's `value` is the result's JSON.
   */
  verifyPath(path: string, options: string): Outcome;
  /** As `verifyPath`, the workbook being `data`, read in place for the call. */
  verifyBytes(data: Buffer, options: string): Outcome;
  /** Checks `policy`, the policy's JSON object alone; the outcome's `value` is the result's JSON. */
  checkPolicy(policy: string): Outcome;
  Book: {
    openPath(path: string): AddonBook;
    openBytes(data: Buffer): AddonBook;
  };
}

/** One loaded engine addon. */
export interface Loaded {
  path: string;
  addon: Addon;
}

export interface OpenOptions {
  path?: string | undefined;
  env?: Readonly<Record<string, string | undefined>>;
  /** The addon the platform's package holds, or null for none. */
  packaged?: string | null;
}

/**
 * The addon `env` or `path` names, or the packaged copy.
 *
 * Throws {@link LibraryNotFoundError} when no addon is found, it cannot be
 * loaded, or it does not export what this package calls.
 */
export function open({
  path,
  env = process.env,
  packaged = packagedAddon(),
}: OpenOptions = {}): Loaded {
  // An empty value counts as unset, as it does for the other SDKs.
  const found = env[ENV_VAR] || path || packaged;
  if (!found) {
    throw new LibraryNotFoundError(installHint('no engine addon was found'));
  }

  const addon = load(found);
  if (typeof addon?.version !== 'function') {
    throw new LibraryNotFoundError(`${found} exports no \`version\``);
  }
  return { path: found, addon };
}

// Resolved against the working directory, as a path a user types is; the
// require() is anchored at this module and would resolve a relative path
// against it. Never unloaded: Node.js cannot unload an addon.
function load(found: string): Addon {
  try {
    return createRequire(import.meta.url)(resolve(found)) as Addon;
  } catch (error) {
    throw new LibraryNotFoundError(installHint(`${found} could not be loaded (${error})`));
  }
}

function installHint(reason: string): string {
  const platformName = platformPackage();
  const own = platformName
    ? `install ${platformName}, which npm installs with this package unless optional packages are omitted`
    : `no package is built for ${process.platform} ${process.arch}`;
  return (
    `${reason}.\n\n` +
    'This package never downloads the engine. Point it at one of these ways:\n' +
    `  * set ${ENV_VAR} to the path of an xlsx_graft.node addon\n` +
    '  * pass version({ addon: "/path/to/xlsx_graft.node" })\n' +
    `  * ${own}`
  );
}
