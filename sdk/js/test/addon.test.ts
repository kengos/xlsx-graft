import { copyFileSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';
import { open } from '../src/addon.js';
import { LibraryNotFoundError, XlsxGraftError } from '../src/index.js';
import { engineAddon, engineVersion, PACKAGE_ROOT } from './support/engine.js';

function scratch(): string {
  return mkdtempSync(join(tmpdir(), 'xlsx-graft-'));
}

const nowhere = '/nowhere/xlsx_graft.node';

describe('open', () => {
  it('takes XLSX_GRAFT_ADDON over the path option', () => {
    const env = { XLSX_GRAFT_ADDON: engineAddon() };
    expect(open({ path: nowhere, env, packaged: nowhere }).path).toBe(engineAddon());
  });

  it('takes the path option when XLSX_GRAFT_ADDON is empty', () => {
    const env = { XLSX_GRAFT_ADDON: '' };
    expect(open({ path: engineAddon(), env, packaged: nowhere }).path).toBe(engineAddon());
  });

  it('resolves a relative path against the working directory', () => {
    const path = relative(process.cwd(), engineAddon());
    expect(open({ path, env: {}, packaged: nowhere }).path).toBe(path);
  });

  it('falls back to the packaged copy', () => {
    const packaged = join(scratch(), 'xlsx_graft.node');
    copyFileSync(engineAddon(), packaged);
    expect(open({ env: {}, packaged }).path).toBe(packaged);
  });

  it('throws with the ways to install when nothing is found', () => {
    expect(() => open({ env: {}, packaged: nowhere })).toThrow(
      /no engine addon was found.*XLSX_GRAFT_ADDON.*\/nowhere\/xlsx_graft\.node/s,
    );
  });

  it('looks for the packaged copy beside dist/ by default', () => {
    expect(() => open({ env: {} })).toThrow(join(PACKAGE_ROOT, 'native', 'xlsx_graft.node'));
  });

  it('throws naming the file when it is not an addon', () => {
    const notAnAddon = join(scratch(), 'not_an_addon.node');
    writeFileSync(notAnAddon, 'text');
    expect(() => open({ path: notAnAddon, env: {}, packaged: nowhere })).toThrow(
      /not_an_addon\.node could not be loaded/,
    );
  });

  it('throws when the addon does not export version', () => {
    const path = join(PACKAGE_ROOT, 'test', 'fixtures', 'no-version.cjs');
    expect(() => open({ path, env: {}, packaged: nowhere })).toThrow(/exports no `version`/);
  });

  it('throws a LibraryNotFoundError, which is an XlsxGraftError', () => {
    const error = (() => {
      try {
        open({ env: {}, packaged: nowhere });
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
