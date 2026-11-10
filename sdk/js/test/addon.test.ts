import { copyFileSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';
import { open, packagedAddon, platformPackage } from '../src/addon.js';
import { LibraryNotFoundError, XlsxGraftError } from '../src/index.js';
import { engineAddon, engineVersion, PACKAGE_ROOT } from './support/engine.js';

function scratch(): string {
  return mkdtempSync(join(tmpdir(), 'xlsx-graft-'));
}

const nowhere = '/nowhere/xlsx_graft.node';

describe('open', () => {
  it('takes XLSX_GRAFT_ADDON over the path option', () => {
    const env = { XLSX_GRAFT_ADDON: engineAddon() };
    expect(open({ path: nowhere, env, packaged: null }).path).toBe(engineAddon());
  });

  it('takes the path option when XLSX_GRAFT_ADDON is empty', () => {
    const env = { XLSX_GRAFT_ADDON: '' };
    expect(open({ path: engineAddon(), env, packaged: null }).path).toBe(engineAddon());
  });

  it('resolves a relative path against the working directory', () => {
    const path = relative(process.cwd(), engineAddon());
    expect(open({ path, env: {}, packaged: null }).path).toBe(path);
  });

  it('falls back to the addon of the platform package', () => {
    const packaged = join(scratch(), 'xlsx_graft.node');
    copyFileSync(engineAddon(), packaged);
    expect(open({ env: {}, packaged }).path).toBe(packaged);
  });

  it('throws with the ways to install when nothing is found', () => {
    expect(() => open({ env: {}, packaged: null })).toThrow(
      /no engine addon was found.*XLSX_GRAFT_ADDON.*install @xlsx-graft\//s,
    );
  });

  it('says so when no package is built for the platform', () => {
    const original = Object.getOwnPropertyDescriptor(process, 'platform') as PropertyDescriptor;
    Object.defineProperty(process, 'platform', { value: 'freebsd' });
    try {
      expect(() => open({ env: {}, packaged: null })).toThrow(/no package is built for freebsd/);
    } finally {
      Object.defineProperty(process, 'platform', original);
    }
  });

  it('finds no addon by default, as the checkout installs no platform package', () => {
    expect(() => open({ env: {} })).toThrow(/no engine addon was found/);
  });

  it('throws naming the file when it is not an addon', () => {
    const notAnAddon = join(scratch(), 'not_an_addon.node');
    writeFileSync(notAnAddon, 'text');
    expect(() => open({ path: notAnAddon, env: {}, packaged: null })).toThrow(
      /not_an_addon\.node could not be loaded/,
    );
  });

  it('throws when the addon does not export version', () => {
    const path = join(PACKAGE_ROOT, 'test', 'fixtures', 'no-version.cjs');
    expect(() => open({ path, env: {}, packaged: null })).toThrow(/exports no `version`/);
  });

  it('throws a LibraryNotFoundError, which is an XlsxGraftError', () => {
    const error = (() => {
      try {
        open({ env: {}, packaged: null });
      } catch (thrown) {
        return thrown;
      }
    })();
    expect(error).toBeInstanceOf(LibraryNotFoundError);
    expect(error).toBeInstanceOf(XlsxGraftError);
    expect((error as Error).name).toBe('LibraryNotFoundError');
  });
});

describe('the real addon', () => {
  // Pins the object the shim (engine/napi/src/shim.rs) hands over: the status
  // and the value, and no other key. The value's absence on a status other
  // than OK cannot be reached through the real addon, whose version call
  // always succeeds; napi-rs leaves an unset `Option` field off the object
  // (`Outcome.value` is optional, never null).
  it('returns the status and the value, and nothing else', () => {
    const { addon } = open({ env: { XLSX_GRAFT_ADDON: engineAddon() } });
    expect(addon.version()).toStrictEqual({ status: 0, value: engineVersion() });
  });
});

describe('platformPackage', () => {
  it.each([
    ['linux', 'x64', '@xlsx-graft/linux-x64-gnu'],
    ['linux', 'arm64', '@xlsx-graft/linux-arm64-gnu'],
    ['darwin', 'x64', '@xlsx-graft/darwin-x64'],
    ['darwin', 'arm64', '@xlsx-graft/darwin-arm64'],
    ['win32', 'x64', '@xlsx-graft/win32-x64-msvc'],
    ['win32', 'arm64', null],
    ['linux', 'ia32', null],
    ['freebsd', 'x64', null],
  ])('names the package for %s %s', (platform, arch, expected) => {
    expect(platformPackage(platform, arch)).toBe(expected);
  });

  it('names the package of every platform a release builds', () => {
    // The release names each platform's package by the table's npm column; the
    // SDK looks it up by its own mapping.
    const table = readFileSync(
      join(PACKAGE_ROOT, '..', '..', 'scripts', 'release', 'platforms.tsv'),
      'utf8',
    )
      .split('\n')
      .filter((line) => line !== '' && !line.startsWith('#'))
      .map((line) => line.split('\t'));
    const [header, ...rows] = table;
    const slug = header?.indexOf('slug') as number;
    const npm = header?.indexOf('npm') as number;
    expect(rows).toHaveLength(5);
    for (const row of rows) {
      const [platform, arch] = (row[slug] as string).split('-');
      expect(platformPackage(platform, arch)).toBe(row[npm]);
    }
  });

  it('names the package of the machine the tests run on, when there is one', () => {
    expect(platformPackage()).toBe(platformPackage(process.platform, process.arch));
  });
});

describe('packagedAddon', () => {
  it('resolves xlsx_graft.node in the platform package', () => {
    const resolved: string[] = [];
    const found = packagedAddon('@xlsx-graft/linux-x64-gnu', (id) => {
      resolved.push(id);
      return `/node_modules/${id}`;
    });
    expect(resolved).toEqual(['@xlsx-graft/linux-x64-gnu/xlsx_graft.node']);
    expect(found).toBe('/node_modules/@xlsx-graft/linux-x64-gnu/xlsx_graft.node');
  });

  it('is null when the package is not installed', () => {
    expect(
      packagedAddon('@xlsx-graft/linux-x64-gnu', () => {
        throw new Error('Cannot find module');
      }),
    ).toBeNull();
  });

  it('is null when no package is built for the platform', () => {
    expect(packagedAddon(null)).toBeNull();
  });

  it('looks the package up from this package, and finds none in the checkout', () => {
    expect(packagedAddon()).toBeNull();
  });
});
