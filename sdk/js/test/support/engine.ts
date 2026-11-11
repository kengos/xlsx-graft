/** What every test shares: the engine addon and the engine's version. */

import { readFileSync, realpathSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ENV_VAR } from '../../src/addon.js';

/** sdk/js, the package's root. */
export const PACKAGE_ROOT = fileURLToPath(new URL('../../', import.meta.url));

/**
 * The repository's `fixtures/` workbooks. The gate copies sdk/js beside a
 * link to the engine, so the folder is found from the link's target.
 */
export function fixture(name: string): string {
  const engine = realpathSync(join(PACKAGE_ROOT, '..', '..', 'engine'));
  return join(engine, '..', 'fixtures', name);
}

/** The addon `make engine:napi` built; the gate points XLSX_GRAFT_ADDON at it. */
export function engineAddon(): string {
  const path = process.env[ENV_VAR];
  if (!path) {
    throw new Error(`${ENV_VAR} is unset; run the tests through make sdk:test:js`);
  }
  return path;
}

/** The engine workspace's version, from engine/Cargo.toml. */
export function engineVersion(): string {
  const cargoToml = readFileSync(new URL('../../../../engine/Cargo.toml', import.meta.url), 'utf8');
  const found = /^\[workspace\.package\][^[]*?^version = "([^"]+)"/ms.exec(cargoToml);
  if (!found) {
    throw new Error('engine/Cargo.toml has no [workspace.package] version');
  }
  return found[1] as string;
}
