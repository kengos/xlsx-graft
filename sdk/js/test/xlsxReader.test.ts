import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  type Book,
  CalendarDate,
  CellError,
  ClosedError,
  LibraryNotFoundError,
  type Result,
  type Row,
  SheetInfo,
  TimeOfDay,
  XlsxReader,
} from '../src/index.js';
import { engineAddon, PACKAGE_ROOT } from './support/engine.js';
import * as workbook from './support/workbook.js';

const DATA =
  '<row r="1"><c r="A1" t="s"><v>1</v></c><c r="C1"><v>3</v></c>' +
  '<c r="D1" t="b"><v>1</v></c></row>' +
  '<row r="3"><c r="B3" s="1"><v>45322</v></c><c r="C3" s="2"><v>0.75</v></c>' +
  '<c r="D3" s="3"><v>45322.000005787037</v></c><c r="E3" s="4"><v>1.5</v></c>' +
  '<c r="F3" t="e"><v>#N/A</v></c><c r="G3" t="inlineStr"><is><t></t></is></c></row>' +
  '<row r="4"/>';
const BYTES = workbook.xlsx(
  workbook.worksheet('Data', DATA),
  workbook.chartsheet('Chart'),
  workbook.worksheet('Hidden', '', 'veryHidden'),
);

function rowsOf(data: Uint8Array, which: string | number = 0, timezone = 'UTC'): Result<Row[]> {
  return XlsxReader.openBytes(data, { timezone }, (book) => [...book.sheet(which).rows()]);
}

/** A workbook whose sheet holds the date-time `serial` in A1. */
function moment(serial: number): Buffer {
  const row = `<row r="1"><c r="A1" s="3"><v>${serial}</v></c></row><row r="2"/>`;
  return workbook.xlsx(workbook.worksheet('Data', row));
}

function scratch(): string {
  return mkdtempSync(join(tmpdir(), 'xlsx-graft-'));
}

afterEach(() => {
  vi.unstubAllEnvs();
});

describe('XlsxReader.openBytes', () => {
  it("gives each row with its number and its values from column A, and the callback's value", () => {
    const result = rowsOf(BYTES, 'data');
    expect(result.success).toBe(true);
    expect([result.error, result.diagnostics]).toEqual([null, []]);
    const rows = result.value as Row[];
    expect(rows.map((row) => row.number)).toEqual([1, 3, 4]);
    expect(rows[0]?.values).toEqual(['b', null, 3, true]);
    expect(rows[1]?.values).toStrictEqual([
      null,
      new CalendarDate(2024, 1, 31),
      new TimeOfDay(18, 0, 0, 0),
      new Date('2024-01-31T00:00:00.500Z'),
      36 * 3_600_000,
      new CellError('#N/A'),
      '',
    ]);
    expect(rows[2]?.values).toEqual([]);
    expect(Object.isFrozen(rows[0]) && Object.isFrozen(rows[0]?.values)).toBe(true);
  });

  it('places a date-time in the zone the caller gives', () => {
    for (const timezone of ['Asia/Tokyo', '+09:00']) {
      const values = rowsOf(BYTES, 0, timezone).value?.[1]?.values;
      expect(values?.[3]).toEqual(new Date('2024-01-30T15:00:00.500Z'));
    }
  });

  it.each([
    [45361.10416666667, '2024-03-10 02:30:00.000', 'its daylight saving skips it'],
    [45599.0625, '2024-11-03 01:30:00.000', 'it has it twice'],
  ])(
    'fails with datetime_unrepresentable for %d, which the zone cannot place',
    (serial, wall, why) => {
      const result = rowsOf(moment(serial), 0, 'America/New_York');
      expect([result.success, result.value, result.diagnostics]).toEqual([false, null, []]);
      expect(result.error?.code).toBe('datetime_unrepresentable');
      expect(result.error?.message).toBe(
        `row 1: ${wall} cannot be placed in "America/New_York": ${why}`,
      );
    },
  );

  it('stops the sheet at a date-time the zone cannot place, the rows before it having come', () => {
    const rows = `<row r="1"/><row r="2"><c r="A2" s="3"><v>45361.10416666667</v></c></row>`;
    const data = workbook.xlsx(workbook.worksheet('Data', rows));
    const read: number[] = [];
    XlsxReader.openBytes(data, { timezone: 'America/New_York' }, (book) => {
      for (const row of book.sheet().rows()) {
        read.push(row.number);
      }
    });
    expect(read).toEqual([1]);
  });

  it('lists the sheets', () => {
    expect(XlsxReader.openBytes(BYTES, (book) => book.sheets()).value).toStrictEqual([
      new SheetInfo(0, 'Data', 'worksheet', 'visible'),
      new SheetInfo(1, 'Chart', 'chartsheet', 'visible'),
      new SheetInfo(2, 'Hidden', 'worksheet', 'veryHidden'),
    ]);
  });

  it('reads a sheet again from its start', () => {
    const result = XlsxReader.openBytes(BYTES, (book) => {
      const sheet = book.sheet('Data');
      const first = sheet.rows().next().value as Row;
      return [first.number, ...[...sheet.rows()].map((row) => row.number)];
    });
    expect(result.value).toEqual([1, 1, 3, 4]);
  });

  it('reads two sheets in turns', () => {
    const result = XlsxReader.openBytes(BYTES, (book) => {
      const first = book.sheet().rows();
      const second = book.sheet('Data').rows();
      const pairs = [];
      for (const row of first) {
        pairs.push([row.number, second.next().value?.number]);
      }
      return pairs;
    });
    expect(result.value).toEqual([
      [1, 1],
      [3, 3],
      [4, 4],
    ]);
  });

  it('takes a view of a Uint8Array and copies its bytes when called', () => {
    const data = new Uint8Array(BYTES.length + 2);
    data.set(BYTES, 1);
    const result = XlsxReader.openBytes(data.subarray(1, -1), (book) => {
      data.fill(0);
      return book.sheets().length;
    });
    expect(result.value).toBe(3);
  });
});

describe('a refused sheet', () => {
  it.each([
    ['Missing', 'sheet_not_found', 'no sheet is named "Missing"'],
    [9, 'sheet_not_found', 'there is no sheet at index 9: the workbook has 3, from index 0'],
    ['Chart', 'sheet_not_worksheet', 'sheet "Chart" is a chartsheet, which holds no cells'],
    [1, 'sheet_not_worksheet', 'sheet "Chart" is a chartsheet, which holds no cells'],
  ])('%s fails reading no rows', (which, code, message) => {
    const read: Row[] = [];
    const result = XlsxReader.openBytes(BYTES, (book) => {
      read.push(...book.sheet(which).rows());
    });
    expect([result.success, result.value, read]).toEqual([false, null, []]);
    expect([result.error?.code, result.error?.severity, result.error?.message]).toEqual([
      code,
      'error',
      message,
    ]);
  });

  it('goes on with the callback and keeps the refusal as the error', () => {
    const result = XlsxReader.openBytes(BYTES, (book) => {
      expect([...book.sheet('Missing').rows()]).toEqual([]);
      return [...book.sheet('Data').rows()].length;
    });
    expect([result.value, result.error?.code]).toEqual([null, 'sheet_not_found']);
  });

  it('fails with sheet_unreadable when the sheet part is not known', () => {
    const result = rowsOf(workbook.xlsx(workbook.unrelated('Lost')), 'Lost');
    expect(result.error?.code).toBe('sheet_unreadable');
    expect(result.error?.message).toBe('sheet "Lost" cannot be read; the diagnostics say why');
    expect(result.diagnostics).not.toEqual([]);
  });

  it('fails with sheet_ambiguous when two sheets have the name', () => {
    const twice = workbook.xlsx(workbook.worksheet('Data'), workbook.worksheet('DATA'));
    const result = rowsOf(twice, 'data');
    expect(result.error?.message).toBe('more than one sheet is named "data", letter case aside');
    expect(result.diagnostics.map((d) => d.code)).toContain('sheet_name_duplicate');
  });

  it('fails with invalid_arg for a name that is not UTF-8', () => {
    expect(rowsOf(BYTES, '\udcff').error?.code).toBe('invalid_arg');
  });

  it('fails with workbook_unreadable, listing no sheets, when the bytes are no workbook', () => {
    const result = XlsxReader.openBytes(Buffer.from('not a zip'), (book) => [
      book.sheets(),
      [...book.sheet().rows()],
    ]);
    expect(result.error?.code).toBe('workbook_unreadable');
    expect(result.error?.message).toMatch(/^the workbook cannot be read/);
    expect(result.diagnostics).not.toEqual([]);
  });
});

it('fails without an error when a problem stops the reading', () => {
  const twice = '<row r="1"><c r="A1"><v>1</v></c></row><row r="1"/>';
  const read: number[] = [];
  const result = XlsxReader.openBytes(workbook.xlsx(workbook.worksheet('Data', twice)), (book) => {
    for (const row of book.sheet().rows()) {
      read.push(row.number);
    }
  });
  expect(read).toEqual([1]);
  expect([result.success, result.value, result.error]).toEqual([false, null, null]);
  const [first] = result.diagnostics;
  expect([first?.code, first?.severity]).toEqual(['cell_order_invalid', 'error']);
  expect(first?.location.part).toMatch(/sheet1\.xml$/);
  expect(first?.decidedBy).toMatchObject({ kind: 'standard' });
});

describe('the book after its callback', () => {
  it('is closed when the callback throws, which throws on', () => {
    let kept: Book | undefined;
    expect(() =>
      XlsxReader.openBytes(BYTES, (book) => {
        kept = book;
        throw new Error('boom');
      }),
    ).toThrow('boom');
    expect(() => kept?.sheets()).toThrow(ClosedError);
    expect(() => kept?.sheet('Data')).toThrow(/the book is closed/);
  });

  it('throws a ClosedError for rows read after it', () => {
    const { value } = XlsxReader.openBytes(BYTES, (book) => {
      const sheet = book.sheet();
      const rows = sheet.rows();
      rows.next();
      return { sheet, rows };
    });
    expect(() => value?.sheet.rows().next()).toThrow(ClosedError);
    expect(() => value?.rows.next()).toThrow(ClosedError);
  });

  it('lets an iterator left open be returned after', () => {
    const result = XlsxReader.openBytes(BYTES, (book) => {
      const rows = book.sheet().rows();
      rows.next();
      return rows;
    });
    expect(result.success).toBe(true);
    expect(result.value?.return()).toEqual({ done: true, value: undefined });
  });
});

describe('a mistake in the caller', () => {
  it.each([[1.5], [-1], [2 ** 32], [Number.NaN]])(
    'throws a RangeError for the index %d',
    (which) => {
      XlsxReader.openBytes(BYTES, (book) => {
        expect(() => book.sheet(which)).toThrow(RangeError);
      });
    },
  );

  it.each([[true], [null], [Buffer.from('Data')]])(
    'throws a TypeError for the sheet %s',
    (which) => {
      XlsxReader.openBytes(BYTES, (book) => {
        expect(() => book.sheet(which as never)).toThrow(TypeError);
      });
    },
  );

  it('throws a TypeError without a callback', () => {
    expect(() => XlsxReader.openBytes(BYTES, {} as never)).toThrow(/a callback is required/);
  });

  it('throws a TypeError, the book closed, for a callback that returns a promise', () => {
    let kept: Book | undefined;
    expect(() =>
      XlsxReader.openBytes(BYTES, async (book) => {
        kept = book;
        await null;
        book.sheets();
      }),
    ).toThrow(/returned a promise/);
    expect(() => kept?.sheets()).toThrow(ClosedError);
  });

  it('throws a RangeError for a zone Intl does not take', () => {
    expect(() => XlsxReader.openBytes(BYTES, { timezone: 'Mars/Olympus' }, () => 1)).toThrow(
      /timezone "Mars\/Olympus" is no zone Intl takes/,
    );
  });
});

describe('XlsxReader.open', () => {
  it('reads the workbook at a path or a file URL', () => {
    const path = join(scratch(), 'book.xlsx');
    writeFileSync(path, BYTES);
    for (const where of [path, pathToFileURL(path)]) {
      const result = XlsxReader.open(where, undefined, (book) => [...book.sheet().rows()].length);
      expect([result.success, result.value]).toEqual([true, 3]);
    }
  });

  it("fails with io, the host's reason, not calling the callback, when the file cannot be opened", () => {
    const read = vi.fn();
    const result = XlsxReader.open('/nowhere/book.xlsx', read);
    expect(read).not.toHaveBeenCalled();
    expect([result.success, result.diagnostics, result.error?.code]).toEqual([false, [], 'io']);
    expect(result.error?.message).toContain('nowhere');
  });

  it('fails with invalid_arg, not calling the callback, for a path that is not UTF-8', () => {
    const read = vi.fn();
    expect(XlsxReader.open('/tmp/\udcff.xlsx', read).error?.code).toBe('invalid_arg');
    expect(read).not.toHaveBeenCalled();
  });

  it('loads the addon option when XLSX_GRAFT_ADDON is unset', () => {
    const addon = engineAddon();
    vi.stubEnv('XLSX_GRAFT_ADDON', '');
    expect(XlsxReader.openBytes(BYTES, { addon }, () => 'read').value).toBe('read');
  });

  it('throws a LibraryNotFoundError for an addon that exports no Book', () => {
    vi.stubEnv('XLSX_GRAFT_ADDON', join(PACKAGE_ROOT, 'test', 'fixtures', 'version-only.cjs'));
    expect(() => XlsxReader.open('book.xlsx', () => 1)).toThrow(LibraryNotFoundError);
  });
});
