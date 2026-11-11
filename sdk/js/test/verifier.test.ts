import { copyFileSync, mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  Diagnostic,
  LibraryNotFoundError,
  PolicyInvalidError,
  type Result,
  Verifier,
  type VerifyRules,
} from '../src/index.js';
import { fixture, PACKAGE_ROOT } from './support/engine.js';

const MACRO = fixture('macro.xlsm');
const TABLE = fixture('table.xlsx');
const DYNAMIC = fixture('dynamic-array.xlsx');
const MAX_OPTIONS_BYTES = 2 << 20;

function codes(result: Result<null>): string[] {
  return result.diagnostics.map((d) => d.code);
}

afterEach(() => {
  vi.unstubAllEnvs();
});

describe('Verifier#verify', () => {
  const verifier = new Verifier();

  it('fails a workbook the engine refuses, with its diagnostics, no error and no value', () => {
    const result = verifier.verify(MACRO);
    expect([result.success, result.error, result.value]).toEqual([false, null, null]);
    expect(codes(result)).toEqual(['part_denied', 'part_denied']);
    expect(result.diagnostics[0]).toMatchObject({
      severity: 'error',
      decidedBy: expect.any(Object),
      fix: { in: 'template' },
    });
  });

  it('passes a workbook whose denied part is allowed, and takes a file: URL', () => {
    const allowed = verifier.verify(pathToFileURL(MACRO), { allowParts: ['macro'] });
    expect([allowed.success, allowed.diagnostics]).toEqual([true, []]);
    const table = verifier.verify(TABLE);
    expect([table.success, table.diagnostics, table.value]).toEqual([true, [], null]);
  });

  it('fails with an io error naming the path for a file that cannot be read', () => {
    const result = verifier.verify('/nowhere/book.xlsx');
    expect([result.success, result.diagnostics]).toEqual([false, []]);
    expect(result.error).toMatchObject({
      code: 'io',
      message: expect.stringContaining('/nowhere/book.xlsx'),
    });
  });

  it('reports a fault of the rules as diagnostics and does not open the workbook', () => {
    const result = verifier.verify('/nowhere/book.xlsx', {
      targets: ['excel 1990'],
      allowFunctions: ['_xlfn.SEQUENCE'],
    });
    expect([result.success, result.error]).toEqual([false, null]);
    expect(codes(result).sort()).toEqual(['rules_invalid', 'target_invalid']);
    const rules = result.diagnostics.find((d) => d.code === 'rules_invalid');
    expect(rules?.args).toMatchObject({ reason: 'prefixed' });
  });

  it('treats a rule that is null as left out', () => {
    const rules: VerifyRules = {
      targets: null,
      bestEffort: null,
      allowFunctions: null,
      allowParts: null,
      failOn: null,
    };
    expect(verifier.verify(TABLE, rules).success).toBe(true);
  });

  it('fails on a warning only when failOn says so', () => {
    const lenient = verifier.verify(DYNAMIC, { bestEffort: ['excel 2016'] });
    expect(lenient.success).toBe(true);
    expect(lenient.diagnostics.map((d) => d.severity)).toContain('warn');
    const strict = verifier.verify(DYNAMIC, { bestEffort: ['excel 2016'], failOn: 'warn' });
    expect([strict.success, strict.error]).toEqual([false, null]);
    expect(strict.diagnostics.map((d) => d.severity)).not.toContain('error');
  });

  it('for a path with a lone surrogate, reads the file whose name has U+FFFD in its place, and verifies its bytes', () => {
    const dir = mkdtempSync(join(tmpdir(), 'xlsx-graft-'));
    copyFileSync(TABLE, join(dir, '\uFFFD.xlsx'));
    expect(verifier.verify(join(dir, '\uD800.xlsx')).success).toBe(true);
    const missing = verifier.verify(join(dir, '\uD800-missing.xlsx'));
    expect([missing.success, missing.diagnostics]).toEqual([false, []]);
    expect(missing.error?.code).toBe('io');
    expect(missing.error?.message).toMatch(/^the file could not be read: /);
  });

  it('writes a lone surrogate of a rule as U+FFFD, and the engine reports the rule', () => {
    const result = verifier.verify(TABLE, { targets: ['\uD800'] });
    expect([result.success, result.error]).toEqual([false, null]);
    expect(codes(result)).toEqual(['target_invalid']);
  });

  it('throws a TypeError for a rule JSON cannot write', () => {
    const rules = { targets: [1n as unknown as string] };
    expect(() => verifier.verify(TABLE, rules)).toThrow(TypeError);
  });

  it('calls the engine with options of exactly 2 MiB, and throws a RangeError for one byte more', () => {
    const base = Buffer.byteLength('{"policy":null,"rules":{"targets":[""]}}');
    const exact = { targets: ['A'.repeat(MAX_OPTIONS_BYTES - base)] };
    const result = verifier.verify(TABLE, exact);
    expect(codes(result)).toEqual(['target_invalid']);
    const over = { targets: ['A'.repeat(MAX_OPTIONS_BYTES - base + 1)] };
    expect(() => verifier.verify(TABLE, over)).toThrow(RangeError);
    expect(() => verifier.verify(TABLE, over)).toThrow(/the engine takes 2097152/);
  });

  it('counts the bytes of the JSON in UTF-8, not its characters', () => {
    const base = Buffer.byteLength('{"policy":null,"rules":{"targets":[""]}}');
    const characters = Math.floor((MAX_OPTIONS_BYTES - base) / 3) + 1;
    const targets = ['あ'.repeat(characters)];
    expect(characters + base).toBeLessThan(MAX_OPTIONS_BYTES);
    expect(() => verifier.verify(TABLE, { targets })).toThrow(RangeError);
  });

  it('throws a TypeError for a URL that is not a file: URL', () => {
    expect(() => verifier.verify(new URL('https://example.com/book.xlsx'))).toThrow(TypeError);
  });
});

describe('Verifier#verifyBytes', () => {
  const verifier = new Verifier();

  it('verifies the bytes; bytes that are not a workbook fail with a diagnostic and no error', () => {
    expect(verifier.verifyBytes(readFileSync(TABLE)).success).toBe(true);
    const result = verifier.verifyBytes(new Uint8Array());
    expect([result.success, result.error]).toEqual([false, null]);
    expect(result.diagnostics).not.toHaveLength(0);
  });

  it('reads only the bytes a view covers', () => {
    const book = readFileSync(MACRO);
    const padded = new Uint8Array(book.length + 10);
    padded.set(book, 4);
    const result = verifier.verifyBytes(padded.subarray(4, 4 + book.length));
    expect(codes(result)).toEqual(['part_denied', 'part_denied']);
  });

  it('takes the rules of the call', () => {
    const result = verifier.verifyBytes(readFileSync(MACRO), { allowParts: ['macro'] });
    expect(result.success).toBe(true);
  });
});

describe('new Verifier', () => {
  it('is frozen, and verifies under the engine defaults for a null policy', () => {
    const verifier = new Verifier({ policy: null });
    expect(Object.isFrozen(verifier)).toBe(true);
    expect(verifier.verify(TABLE).success).toBe(true);
  });

  it('writes the policy to the engine as given, so its maxEntries fails the verification', () => {
    const result = new Verifier({ policy: { maxEntries: 1 } }).verify(TABLE);
    expect(result.success).toBe(false);
    expect(result.diagnostics[0]?.decidedBy).toMatchObject({ source: 'policy' });
  });

  it("fails on a warning when the policy's failOn is warn", () => {
    const verifier = new Verifier({ policy: { failOn: 'warn' } });
    expect(verifier.verify(DYNAMIC, { bestEffort: ['excel 2016'] }).success).toBe(false);
  });

  it('keeps the policy it checked when the caller changes theirs', () => {
    const policy = { maxEntries: 1 };
    const verifier = new Verifier({ policy });
    policy.maxEntries = 10_000;
    expect(verifier.verify(TABLE).success).toBe(false);
  });

  it("throws PolicyInvalidError with the engine's diagnostics", () => {
    const policy = { maxEntries: 0, forbid: { parts: [{ kind: 'nope' }] } };
    let thrown: unknown;
    try {
      new Verifier({ policy });
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeInstanceOf(PolicyInvalidError);
    const { diagnostics, message } = thrown as PolicyInvalidError;
    expect(diagnostics.map((d) => d.code)).toEqual(['policy_invalid', 'policy_invalid']);
    expect(diagnostics.map((d) => d.args.key)).toEqual(['maxEntries', 'forbid.parts[0]']);
    expect(message.startsWith('the policy is not valid: ')).toBe(true);
    expect(message).toBe(
      `the policy is not valid: ${diagnostics.map((d) => d.message).join('; ')}`,
    );
  });

  it("puts the engine's error before the diagnostics in a PolicyInvalidError", () => {
    vi.stubEnv('XLSX_GRAFT_ADDON', join(PACKAGE_ROOT, 'test', 'fixtures', 'verify-panic.cjs'));
    let thrown: unknown;
    try {
      new Verifier({ policy: {} });
    } catch (error) {
      thrown = error;
    }
    expect((thrown as PolicyInvalidError).diagnostics).toEqual([
      new Diagnostic({
        code: 'panic',
        severity: 'error',
        message: 'the engine panicked; the call did not complete',
      }),
    ]);
  });

  it('does not rename the policy keys: a snake_case key is one the engine does not know', () => {
    let thrown: unknown;
    try {
      new Verifier({ policy: { max_entries: 1 } });
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeInstanceOf(PolicyInvalidError);
    const { diagnostics } = thrown as PolicyInvalidError;
    expect(diagnostics.map((d) => [d.code, d.args.key])).toEqual([
      ['policy_invalid', 'max_entries'],
    ]);
  });

  it('writes a lone surrogate of a policy key or value as U+FFFD, and the engine reports the policy', () => {
    for (const policy of [{ '\uD800': 1 }, { failOn: '\uD800' }]) {
      let thrown: unknown;
      try {
        new Verifier({ policy });
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(PolicyInvalidError);
      const { diagnostics } = thrown as PolicyInvalidError;
      expect(diagnostics.map((d) => d.code)).toEqual(['policy_invalid']);
    }
  });

  it('throws a TypeError for a policy that refers to itself, with or without a lone surrogate in a key', () => {
    const plain: Record<string, unknown> = { maxEntries: 1 };
    plain.self = plain;
    expect(() => new Verifier({ policy: plain })).toThrow(TypeError);
    const lone: Record<string, unknown> = { '\uD800': 1 };
    lone.self = lone;
    expect(() => new Verifier({ policy: lone })).toThrow(TypeError);
  });

  it('throws a TypeError for a policy JSON cannot write', () => {
    expect(() => new Verifier({ policy: { maxEntries: 1n } })).toThrow(TypeError);
  });

  it('throws LibraryNotFoundError for an addon with no verify functions', () => {
    vi.stubEnv('XLSX_GRAFT_ADDON', join(PACKAGE_ROOT, 'test', 'fixtures', 'version-only.cjs'));
    expect(() => new Verifier()).toThrow(LibraryNotFoundError);
    expect(() => new Verifier()).toThrow(/exports no `verifyPath`/);
  });
});
