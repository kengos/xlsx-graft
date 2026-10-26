import { describe, expect, it } from 'vitest';
import type { Addon, Outcome } from '../src/addon.js';
import { Diagnostic, Result } from '../src/index.js';
import * as versionCall from '../src/versionCall.js';

function returning(outcome: Outcome): Addon {
  return { version: () => outcome };
}

describe('versionCall.call', () => {
  it('returns the value on OK', () => {
    expect(versionCall.call(returning({ status: 0, value: '1.2.3' }))).toEqual(Result.ok('1.2.3'));
  });

  it('returns a failure carrying the panic diagnostic on a panic status', () => {
    const result = versionCall.call(returning({ status: 2 }));
    expect(result).toEqual(
      Result.failure([new Diagnostic('panic', 'the engine panicked; the call did not complete')]),
    );
    expect(result.failed).toBe(true);
    expect(result.value).toBeNull();
  });

  it('returns a failure carrying the null_arg diagnostic on a null_arg status', () => {
    const result = versionCall.call(returning({ status: 1 }));
    expect(result.diagnostics.map((d) => d.code)).toEqual(['null_arg']);
  });

  it('returns a failure naming a status this package does not know', () => {
    const [diagnostic] = versionCall.call(returning({ status: 7 })).diagnostics;
    expect(diagnostic?.code).toBe('unknown_status');
    expect(diagnostic?.message).toContain('status 7');
  });
});
