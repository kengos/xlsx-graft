import { describe, expect, it } from 'vitest';
import * as viewOptions from '../src/viewOptions.js';

const MAX_OPTIONS_BYTES = 2 << 20;

describe('viewOptions.build', () => {
  it('writes an empty object for a request that names nothing', () => {
    expect(viewOptions.build({})).toBe('{}');
  });

  it('writes the keys that have a value, in the order sheet, range, maxRows, maxColumns', () => {
    const request = { maxColumns: 3, maxRows: 2, range: 'A1:C3', sheet: 'Data' };
    expect(viewOptions.build(request)).toBe(
      '{"sheet":"Data","range":"A1:C3","maxRows":2,"maxColumns":3}',
    );
  });

  it('writes a sheet that is a number as sheetIndex, 0 included', () => {
    expect(viewOptions.build({ sheet: 0 })).toBe('{"sheetIndex":0}');
    expect(viewOptions.build({ sheet: 2, range: 'A1' })).toBe('{"sheetIndex":2,"range":"A1"}');
  });

  it('leaves out a key that is null', () => {
    const request = { sheet: null, range: null, maxRows: null, maxColumns: null };
    expect(viewOptions.build(request)).toBe('{}');
  });

  it('writes a value of the wrong type as it is, for the engine to refuse', () => {
    const request = { maxRows: 0, maxColumns: Number.POSITIVE_INFINITY, sheet: true };
    expect(viewOptions.build(request as never)).toBe(
      '{"sheet":true,"maxRows":0,"maxColumns":null}',
    );
  });

  it('does not change the request', () => {
    const request = Object.freeze({ sheet: 1, range: 'A1' });
    viewOptions.build(request);
    expect(request).toEqual({ sheet: 1, range: 'A1' });
  });

  it('throws a RangeError for a sheet or a range with a lone surrogate', () => {
    expect(() => viewOptions.build({ sheet: 'a\uD800' })).toThrow(RangeError);
    expect(() => viewOptions.build({ range: '\uDC00' })).toThrow(/lone surrogate/);
    expect(viewOptions.build({ sheet: 'a😀' })).toBe('{"sheet":"a😀"}');
  });

  it('writes options of exactly 2 MiB, and throws a RangeError for one byte more', () => {
    const base = Buffer.byteLength('{"range":""}');
    const exact = viewOptions.build({ range: 'A'.repeat(MAX_OPTIONS_BYTES - base) });
    expect(Buffer.byteLength(exact)).toBe(MAX_OPTIONS_BYTES);
    const over = { range: 'A'.repeat(MAX_OPTIONS_BYTES - base + 1) };
    expect(() => viewOptions.build(over)).toThrow(RangeError);
    expect(() => viewOptions.build(over)).toThrow(/the engine takes 2097152/);
  });

  it('throws a TypeError for a BigInt', () => {
    expect(() => viewOptions.build({ maxRows: 1n as unknown as number })).toThrow(TypeError);
  });
});
