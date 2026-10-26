/**
 * The package downloads nothing and depends on nothing (docs/sdk.md § Finding
 * the engine library).
 */

import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { expect, it } from 'vitest';
import { PACKAGE_ROOT } from './support/engine.js';

const SOURCE_DIR = join(PACKAGE_ROOT, 'src');
const SOURCES = readdirSync(SOURCE_DIR, { recursive: true, encoding: 'utf8' }).filter((name) =>
  name.endsWith('.ts'),
);

// An import, a side-effect import, a dynamic import() or a require() of a
// networking module, or the global networking APIs.
const NETWORKING =
  /(?:\bfrom\s*|\bimport\s*\(?\s*|\brequire\s*\(\s*)['"](?:node:)?(?:http|https|http2|net|tls|dgram|dns|undici)['"]|\bfetch\s*\(|\bWebSocket\b/;

it('reads every source file', () => {
  // A sweep over no files reports no offenders and proves nothing.
  expect(SOURCES).toEqual(expect.arrayContaining(['index.ts', 'addon.ts', 'versionCall.ts']));
});

it('uses no networking API', () => {
  const offenders = SOURCES.filter((name) =>
    NETWORKING.test(readFileSync(join(SOURCE_DIR, name), 'utf8')),
  );
  expect(offenders).toEqual([]);
});

it('matches a networking import with the sweep', () => {
  expect(NETWORKING.test("import { request } from 'node:https';")).toBe(true);
  expect(NETWORKING.test("import net from 'net';")).toBe(true);
  expect(NETWORKING.test("import 'node:tls';")).toBe(true);
  expect(NETWORKING.test("await import('node:http')")).toBe(true);
  expect(NETWORKING.test("require('dns')")).toBe(true);
  expect(NETWORKING.test('await fetch(url)')).toBe(true);
  expect(NETWORKING.test("import { open } from './addon.js';")).toBe(false);
});

it('declares no runtime dependencies and no install hooks', () => {
  const manifest = JSON.parse(readFileSync(join(PACKAGE_ROOT, 'package.json'), 'utf8'));
  for (const field of ['dependencies', 'optionalDependencies', 'peerDependencies']) {
    expect(manifest[field]).toBeUndefined();
  }
  for (const hook of ['preinstall', 'install', 'postinstall', 'prepare']) {
    expect(manifest.scripts[hook]).toBeUndefined();
  }
});
