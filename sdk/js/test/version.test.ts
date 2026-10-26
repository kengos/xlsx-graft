import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Result, version } from '../src/index.js';
import { engineAddon, engineVersion, PACKAGE_ROOT } from './support/engine.js';

function packageJson(): Record<string, unknown> {
  return JSON.parse(readFileSync(join(PACKAGE_ROOT, 'package.json'), 'utf8'));
}

afterEach(() => {
  vi.unstubAllEnvs();
});

describe('version', () => {
  it('returns the engine version from the addon', () => {
    const result = version();
    expect(result).toEqual(Result.ok(engineVersion()));
    expect(result.failed).toBe(false);
  });

  it('loads the addon option when XLSX_GRAFT_ADDON is unset', () => {
    const addon = engineAddon();
    vi.stubEnv('XLSX_GRAFT_ADDON', '');
    expect(version({ addon }).value).toBe(engineVersion());
    expect(() => version({ addon: '/elsewhere/xlsx_graft.node' })).toThrow(
      /\/elsewhere\/xlsx_graft\.node could not be loaded/,
    );
  });
});

describe('the package', () => {
  it('tracks the engine version', () => {
    expect(packageJson().version).toBe(engineVersion());
  });

  it('runs the pnpm the image installs', () => {
    const dockerfile = readFileSync(join(PACKAGE_ROOT, 'Dockerfile'), 'utf8');
    const installed = /npm install -g .*pnpm@(\S+)/.exec(dockerfile)?.[1];
    expect(packageJson().packageManager).toBe(`pnpm@${installed}`);
  });
});
