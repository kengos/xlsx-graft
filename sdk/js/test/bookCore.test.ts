// What the real addon cannot be made to do: a panic, a status this package
// does not know, a value newer than it, diagnostics it cannot read. A fake
// book stands in, read through what XlsxReader.open reads a book with.

import { describe, expect, it } from 'vitest';
import type { AddonBook, AddonCell, AddonRows } from '../src/addon.js';
import { BookCore } from '../src/bookCore.js';
import type { Book, Result } from '../src/index.js';
import { reading } from '../src/xlsxReader.js';
import { Zone } from '../src/zone.js';

const UTC = new Zone('UTC');
const CLEAN = JSON.stringify({ has_errors: false, error: null, diagnostics: [] });

type NextRow = ReturnType<AddonRows['nextRow']>;

/** A book whose calls return what `parts` gives, logging every close. */
function fake(parts: Partial<AddonBook> & { next?: NextRow[] } = {}): {
  book: AddonBook;
  closed: string[];
} {
  const closed: string[] = [];
  const next = [...(parts.next ?? [])];
  const rows = (status: number): AddonRows => ({
    status,
    nextRow: () => next.shift() ?? { status: 0 },
    close: () => closed.push('rows'),
  });
  const book: AddonBook = {
    status: 0,
    sheets: () => ({ status: 0, sheets: [] }),
    rows: () => rows(0),
    rowsNamed: () => rows(0),
    diagnostics: () => ({ status: 0, value: CLEAN }),
    close: () => closed.push('book'),
    ...parts,
  };
  return { book, closed };
}

function row(number: number, ...cells: AddonCell[]): NextRow {
  return { status: 0, row: { number, cells } };
}

/** Reads every row of the first sheet of `book`. */
function readAll(book: AddonBook): Result<number[]> {
  return reading(book, UTC, (opened: Book) => [...opened.sheet().rows()].map((r) => r.number));
}

describe('an open the engine refused', () => {
  it('fails with the status, in the host words when it gives them', () => {
    const io = reading({ ...fake().book, status: 3, message: 'no such file' }, UTC, () => 1);
    expect([io.error?.code, io.error?.message]).toEqual(['io', 'no such file']);
    const panic = reading({ ...fake().book, status: 2, message: null }, UTC, () => 1);
    expect(panic.error?.message).toBe('the engine panicked; the call did not complete');
  });
});

describe('a status other than OK', () => {
  it('keeps the first as the error: a panic, after which every call returns it', () => {
    const { book } = fake({
      sheets: () => ({ status: 2, sheets: [] }),
      rows: () => ({ status: 2, nextRow: () => ({ status: 2 }), close: () => {} }),
      diagnostics: () => ({ status: 2 }),
    });
    const result = reading(book, UTC, (opened) => [opened.sheets(), [...opened.sheet().rows()]]);
    expect([result.success, result.diagnostics, result.error?.code]).toEqual([false, [], 'panic']);
  });

  it('ends the rows on a failing read, saying why the host failed', () => {
    const { book } = fake({ next: [row(1), { status: 3, message: 'disk gone' }] });
    const result = readAll(book);
    expect([result.error?.code, result.error?.message]).toEqual(['io', 'disk gone']);
  });

  it("is said in the engine's words when the engine kept the same", () => {
    const words = { code: 'sheet_not_found', message: 'no sheet is named "X"' };
    const { book } = fake({
      rows: () => ({ status: 6, nextRow: () => ({ status: 0 }), close: () => {} }),
      diagnostics: () => ({
        status: 0,
        value: JSON.stringify({ has_errors: false, error: words, diagnostics: [] }),
      }),
    });
    expect(readAll(book).error?.message).toBe('no sheet is named "X"');
  });

  it('names a status this package does not know', () => {
    const { book } = fake({ sheets: () => ({ status: 42, sheets: [] }) });
    const result = reading(book, UTC, (opened) => opened.sheets());
    expect(result.error?.code).toBe('unknown_status');
    expect(result.error?.message).toContain('status 42');
  });
});

it('stops the rows at a value of a type it does not know, naming the row', () => {
  const { book, closed } = fake({ next: [row(1, { type: 'empty' }), row(2, { type: 'unknown' })] });
  const result = readAll(book);
  expect([result.value, result.error?.code]).toEqual([null, 'unknown_value']);
  expect(result.error?.message).toBe(
    'row 2: the engine returned a value of type unknown, which this package does not know',
  );
  expect(closed).toEqual(['rows', 'book']);
});

describe('the diagnostics', () => {
  const record = {
    code: 'c',
    severity: 'warning',
    message: 'm',
    args: { n: { m: 1 } },
    location: { part: 'p' },
    decidedBy: { rule: 'r' },
    fix: null,
    origin: 'o',
  };

  function readWith(value: string | null, status = 0): Result<number> {
    const outcome = value === null ? { status } : { status, value };
    return reading(fake({ diagnostics: () => outcome }).book, UTC, () => 1);
  }

  it('are the engine’s, every field but origin, and a success has them', () => {
    const json = JSON.stringify({ has_errors: false, error: null, diagnostics: [record] });
    const result = readWith(json);
    expect([result.success, result.value]).toEqual([true, 1]);
    const [diagnostic] = result.diagnostics;
    expect({ ...diagnostic }).toEqual({
      code: 'c',
      severity: 'warning',
      message: 'm',
      args: { n: { m: 1 } },
      location: { part: 'p' },
      decidedBy: { rule: 'r' },
      fix: null,
    });
    expect(Object.isFrozen(diagnostic?.args.n)).toBe(true);
  });

  it('fail the result, with no error, when one is an error', () => {
    const json = JSON.stringify({ has_errors: true, error: null, diagnostics: [record] });
    const result = readWith(json);
    expect([result.success, result.value, result.error]).toEqual([false, null, null]);
  });

  it.each([
    ['not json', /JSON/],
    [JSON.stringify([]), /is not an object/],
    [JSON.stringify({ has_errors: false, error: null }), /no `diagnostics`/],
    [JSON.stringify({ has_errors: false, error: null, diagnostics: {} }), /not an array/],
    [JSON.stringify({ has_errors: false, error: { code: 1 }, diagnostics: [] }), /`code` is not/],
    [
      JSON.stringify({ has_errors: false, error: null, diagnostics: [{ ...record, fix: 1 }] }),
      /1 is not an object/,
    ],
  ])('fail with diagnostics_unreadable when they are %s', (json, why) => {
    const result = readWith(json);
    expect([result.success, result.diagnostics, result.error?.code]).toEqual([
      false,
      [],
      'diagnostics_unreadable',
    ]);
    expect(result.error?.message).toMatch(/^the engine's diagnostics could not be read: /);
    expect(result.error?.message).toMatch(why);
  });

  it('fail with the status when they cannot be had', () => {
    expect(readWith(null, 1).error?.code).toBe('null_arg');
  });
});

it('names a sheet kind or state the package does not know unknown', () => {
  const sheets = [{ name: 'S', kind: 'pivotsheet', state: 'faded' }];
  const { book } = fake({ sheets: () => ({ status: 0, sheets }) });
  const [sheet] = reading(book, UTC, (opened) => opened.sheets()).value ?? [];
  expect([sheet?.kind, sheet?.state]).toEqual(['unknown', 'unknown']);
});

describe('closing', () => {
  it('closes the rows an iterator left open with the book, and them once', () => {
    const { book, closed } = fake({ next: [row(1), row(2)] });
    const result = reading(book, UTC, (opened) => {
      const rows = opened.sheet().rows();
      rows.next();
      return rows;
    });
    expect(closed).toEqual(['book', 'rows']);
    result.value?.return();
    expect(closed).toEqual(['book', 'rows']);
  });

  it('closes the book, then the rows left open, once', () => {
    const { book, closed } = fake();
    const core = new BookCore(book, UTC);
    const rows = core.openRows('Data');
    core.close();
    core.close();
    core.closeRows(rows as AddonRows);
    expect(closed).toEqual(['book', 'rows']);
  });
});
