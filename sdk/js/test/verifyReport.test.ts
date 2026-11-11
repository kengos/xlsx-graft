import { describe, expect, it } from 'vitest';
import type { Outcome } from '../src/addon.js';
import { Diagnostic, Result, XlsxGraftError } from '../src/index.js';
import * as verifyReport from '../src/verifyReport.js';

function json(report: Record<string, unknown>): Outcome {
  const whole = { ok: true, failOn: 'error', counts: {}, error: null, diagnostics: [], ...report };
  return { status: 0, value: JSON.stringify(whole) };
}

const RECORD = {
  code: 'part_denied',
  severity: 'error',
  message: 'the part is denied',
  args: { kind: 'macro' },
  location: { part: 'xl/vbaProject.bin' },
  decidedBy: { source: 'default' },
  fix: { in: 'template' },
};

describe('verifyReport.call', () => {
  it('is successful with no value when ok is true, and keeps the diagnostics', () => {
    const warning = { ...RECORD, severity: 'warn', decidedBy: null, fix: null };
    const result = verifyReport.call(json({ diagnostics: [warning] }));
    expect(result).toEqual(Result.ok(null, [new Diagnostic(warning)]));
    expect(result.success).toBe(true);
  });

  it('is successful when ok is true even if error is broken, which only a failure reads', () => {
    expect(verifyReport.call(json({ error: 'broken' })).success).toBe(true);
    expect(verifyReport.call(json({ error: undefined })).success).toBe(true);
  });

  it('fails with the diagnostics and no error when ok is false and error is null', () => {
    const result = verifyReport.call(json({ ok: false, diagnostics: [RECORD] }));
    expect(result).toEqual(Result.failure([new Diagnostic(RECORD)]));
    expect(result.error).toBeNull();
    expect(Object.isFrozen(result.diagnostics[0]?.args)).toBe(true);
  });

  it('fails with the error the engine gave', () => {
    const error = { code: 'io', message: 'no such file' };
    const result = verifyReport.call(json({ ok: false, error }));
    expect(result.failed).toBe(true);
    expect(result.error).toEqual(new Diagnostic({ ...error, severity: 'error' }));
    expect(result.diagnostics).toEqual([]);
  });

  it('throws an XlsxGraftError for INVALID_ARG, which only a wrongly built call gets', () => {
    expect(() => verifyReport.call({ status: 4 })).toThrow(XlsxGraftError);
    expect(() => verifyReport.call({ status: 4 })).toThrow(
      'the engine refused a call this package built (status 4): a bug of the package',
    );
  });

  it.each([
    [1, 'null_arg'],
    [2, 'panic'],
    [42, 'unknown_status'],
  ])('fails with the %i status named in error, and no diagnostics', (status, code) => {
    const result = verifyReport.call({ status });
    expect(result.failed).toBe(true);
    expect(result.error?.code).toBe(code);
    expect(result.diagnostics).toEqual([]);
  });

  it.each([
    ['is not JSON', { status: 0, value: '{' }],
    ['is not an object', { status: 0, value: '[]' }],
    ['has no ok', json({ ok: undefined })],
    ['has an ok that is not a boolean', json({ ok: 'true' })],
    ['has no diagnostics', json({ diagnostics: undefined })],
    ['has diagnostics that are not an array', json({ diagnostics: {} })],
    ['has a diagnostic that is not an object', json({ diagnostics: [1] })],
    ['has a diagnostic with no code', json({ diagnostics: [{ ...RECORD, code: undefined }] })],
    ['has no error and is not ok', json({ ok: false, error: undefined })],
    ['has an error that is not null or an object, and is not ok', json({ ok: false, error: 'io' })],
    ['has an error with no message, and is not ok', json({ ok: false, error: { code: 'io' } })],
  ])('fails with diagnostics_unreadable when the result %s', (_name, outcome) => {
    const result = verifyReport.call(outcome);
    expect(result.failed).toBe(true);
    expect(result.diagnostics).toEqual([]);
    expect(result.error?.code).toBe('diagnostics_unreadable');
    expect(result.error?.message).toMatch(/^the engine's result could not be read: /);
  });
});
