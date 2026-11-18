import { describe, expect, it } from 'vitest';
import type { Outcome } from '../src/addon.js';
import { Diagnostic, Result, XlsxGraftError } from '../src/index.js';
import * as viewReport from '../src/viewReport.js';

const RECORD = {
  code: 'sheet_not_found',
  severity: 'error',
  message: 'no such sheet',
  args: { sheet: 'Nope' },
  location: {},
  decidedBy: null,
  fix: null,
};

function json(view: Record<string, unknown>): Outcome {
  const whole = { ok: true, error: null, diagnostics: [], value: 'v', ...view };
  return { status: 0, value: JSON.stringify(whole) };
}

const read = (view: Record<string, unknown>): unknown => view.value;

describe('viewReport.call', () => {
  it('is successful with the content and the diagnostics when ok is true, and ignores a broken error', () => {
    const result = viewReport.call(json({ diagnostics: [RECORD], error: 'broken' }), read);
    expect(result).toEqual(Result.ok('v', [new Diagnostic(RECORD)]));
    expect(result.success).toBe(true);
  });

  it('fails with the error, the diagnostics and the content when ok is false', () => {
    const error = { code: 'sheet_not_found', message: 'no such sheet' };
    const result = viewReport.call(json({ ok: false, error, diagnostics: [RECORD] }), read);
    expect([result.success, result.value]).toEqual([false, 'v']);
    expect(result.error).toMatchObject(error);
    expect(result.diagnostics).toEqual([new Diagnostic(RECORD)]);
  });

  it('fails with the content and no error when ok is false and error is null', () => {
    const result = viewReport.call(json({ ok: false }), read);
    expect([result.success, result.value, result.error]).toEqual([false, 'v', null]);
  });

  it('treats a truthy ok as a success and a falsy ok as a failure', () => {
    expect(viewReport.call(json({ ok: 'yes' }), read).success).toBe(true);
    expect(viewReport.call(json({ ok: 0 }), read).success).toBe(false);
  });

  it('ignores a key it does not know', () => {
    expect(viewReport.call(json({ extra: [1] }), read).success).toBe(true);
  });

  it('freezes the diagnostics it keeps', () => {
    const result = viewReport.call(json({ diagnostics: [RECORD] }), read);
    expect(Object.isFrozen(result.diagnostics[0]?.args)).toBe(true);
  });

  it('is a failure with no value and a diagnostics_unreadable error for a result it cannot read', () => {
    const bad: Outcome[] = [
      { status: 0, value: 'not json' },
      { status: 0, value: '[]' },
      json({ diagnostics: 'x' }),
      json({ ok: false, error: 'broken' }),
      { status: 0, value: JSON.stringify({ ok: false, diagnostics: [], value: 'v' }) },
      { status: 0, value: JSON.stringify({ ok: true, error: null, diagnostics: [] }) },
    ];
    for (const outcome of bad) {
      const result = viewReport.call(outcome, (view) => (view.value as unknown[]).length);
      expect([result.success, result.value, result.diagnostics]).toEqual([false, null, []]);
      expect(result.error?.code).toBe('diagnostics_unreadable');
      expect(result.error?.message).toMatch(/^the engine's result could not be read: /);
    }
  });

  it('is a failure for a status that is not OK, with no value and no diagnostics', () => {
    for (const [status, code] of [
      [1, 'null_arg'],
      [2, 'panic'],
      [3, 'io'],
      [99, 'unknown_status'],
    ] as const) {
      const result = viewReport.call({ status }, read);
      expect([result.success, result.value, result.diagnostics]).toEqual([false, null, []]);
      expect(result.error?.code).toBe(code);
    }
  });

  it("throws a RangeError with the engine's message for REQUEST_INVALID", () => {
    expect(() =>
      viewReport.call({ status: 11, message: 'maxRows must be 1 or more' }, read),
    ).toThrow(new RangeError('maxRows must be 1 or more'));
  });

  it('throws a RangeError with a fixed message for REQUEST_INVALID with no message', () => {
    const fixed = 'the engine refused a value of the options (status 11) and gave no reason';
    expect(() => viewReport.call({ status: 11 }, read)).toThrow(new RangeError(fixed));
    expect(() => viewReport.call({ status: 11, message: '' }, read)).toThrow(new RangeError(fixed));
  });

  it('throws an XlsxGraftError for INVALID_ARG', () => {
    expect(() => viewReport.call({ status: 4 }, read)).toThrow(XlsxGraftError);
    expect(() => viewReport.call({ status: 4 }, read)).toThrow(/a bug of the package/);
  });
});
