import { copyFileSync, mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  type Grid,
  type GridCell,
  LibraryNotFoundError,
  SheetInfo,
  type Tree,
  Viewer,
  XlsxGraftError,
} from '../src/index.js';
import { fixture, PACKAGE_ROOT } from './support/engine.js';

const TABLE = fixture('table.xlsx');
const MERGED = fixture('merged.xlsx');
const EXTERNAL = fixture('external-link.xlsx');
const MAX_OPTIONS_BYTES = 2 << 20;

function cell(grid: Grid, ref: string): GridCell | undefined {
  return grid.rows.flatMap((row) => row.cells).find((c) => c.ref === ref);
}

afterEach(() => {
  vi.unstubAllEnvs();
});

describe('Viewer#tree', () => {
  const viewer = new Viewer();

  it('lists the parts of the workbook at a path and its relationships', () => {
    const result = viewer.tree(TABLE);
    expect([result.success, result.error, result.diagnostics]).toEqual([true, null, []]);
    const tree = result.value as Tree;
    expect(tree.complete).toBe(true);
    expect(tree.parts.map((part) => part.name)).toContain('/xl/workbook.xml');
    expect(tree.relationships.map((r) => r.target.part)).toContain('/xl/workbook.xml');
    const workbook = tree.parts.find((part) => part.name === '/xl/workbook.xml');
    expect(workbook).toMatchObject({ readable: true, relationshipsKnown: true });
    expect(workbook?.size).toBeGreaterThan(0);
    expect(workbook?.compressedSize).toBeGreaterThan(0);
    expect(workbook?.relationships.length).toBeGreaterThan(0);
  });

  it('takes a file: URL, and the bytes of the file give the same tree', () => {
    const tree = viewer.tree(TABLE).value;
    expect(viewer.tree(pathToFileURL(TABLE)).value).toEqual(tree);
    expect(viewer.treeBytes(readFileSync(TABLE)).value).toEqual(tree);
  });

  it('gives a target outside the package as external with no part', () => {
    const tree = viewer.tree(EXTERNAL).value as Tree;
    const targets = tree.parts.flatMap((part) => part.relationships.map((r) => r.target));
    const external = targets.find((target) => target.external !== null);
    expect(external?.part).toBeNull();
    expect(typeof external?.external).toBe('string');
  });

  it('freezes the tree and its lists', () => {
    const tree = viewer.tree(TABLE).value as Tree;
    expect(Object.isFrozen(tree)).toBe(true);
    expect(Object.isFrozen(tree.parts)).toBe(true);
    expect(Object.isFrozen(tree.parts[0])).toBe(true);
    expect(Object.isFrozen(tree.relationships)).toBe(true);
    expect(Object.isFrozen(tree.relationships[0]?.target)).toBe(true);
  });

  it('fails with an io error and a tree with no parts for a file that does not exist', () => {
    const result = viewer.tree('/nowhere/book.xlsx');
    expect([result.success, result.error?.code]).toEqual([false, 'io']);
    expect(result.value?.parts).toEqual([]);
  });

  it('fails with diagnostics and a tree with no parts for bytes that are not a workbook', () => {
    const result = viewer.treeBytes(Buffer.from('not a zip'));
    expect(result.success).toBe(false);
    expect(result.diagnostics).not.toHaveLength(0);
    expect(result.value?.parts).toEqual([]);
  });

  it('reads only the bytes the subarray covers', () => {
    const book = readFileSync(TABLE);
    const padded = new Uint8Array(book.length + 10);
    padded.set(book, 4);
    const result = viewer.treeBytes(padded.subarray(4, 4 + book.length));
    expect(result.value).toEqual(viewer.tree(TABLE).value);
  });

  it('for a path with a lone surrogate, reads the file whose name has U+FFFD in its place', () => {
    const dir = mkdtempSync(join(tmpdir(), 'xlsx-graft-'));
    copyFileSync(TABLE, join(dir, '�.xlsx'));
    expect(viewer.tree(join(dir, '\uD800.xlsx')).value).toEqual(viewer.tree(TABLE).value);
    const missing = viewer.tree(join(dir, '\uD800-missing.xlsx'));
    expect([missing.success, missing.value, missing.error?.code]).toEqual([false, null, 'io']);
  });

  it('throws a TypeError for a URL that is not a file: URL', () => {
    expect(() => viewer.tree(new URL('https://example.com/book.xlsx'))).toThrow(TypeError);
  });
});

describe('Viewer#describeSheet', () => {
  const viewer = new Viewer();

  it('shows the first sheet, its ranges and its merges', () => {
    const result = viewer.describeSheet(MERGED);
    expect([result.success, result.error, result.diagnostics]).toEqual([true, null, []]);
    const grid = result.value as Grid;
    expect(grid.sheet).toEqual(new SheetInfo(0, 'Sheet1', 'worksheet', 'visible'));
    expect([grid.dimension, grid.used, grid.shown, grid.complete]).toEqual([
      'A1:E8',
      'A1:E8',
      'A1:E8',
      true,
    ]);
    expect(grid.truncated).toEqual({ rows: false, columns: false, merges: false });
    expect(grid.merges).toEqual(['A1:C1', 'A3:A5']);
  });

  it('shows the cells with the JSON values', () => {
    const grid = viewer.describeSheet(MERGED).value as Grid;
    expect(grid.rows[0]?.row).toBe(1);
    expect(grid.rows[0]?.cells[0]).toEqual({
      ref: 'A1',
      type: 'string',
      value: 'Quarterly report',
      truncated: false,
    });
    expect(cell(grid, 'C3')?.value).toBe(1.5);
    expect(cell(grid, 'C4')?.value).toBe(2);
    expect(cell(grid, 'C6')).toMatchObject({ type: 'date', value: '2024-01-15' });
  });

  it('freezes the grid, its lists and its cells', () => {
    const grid = viewer.describeSheet(MERGED).value as Grid;
    for (const frozen of [grid, grid.truncated, grid.sheet, grid.rows, grid.rows[0]]) {
      expect(Object.isFrozen(frozen)).toBe(true);
    }
    expect(Object.isFrozen(grid.rows[0]?.cells)).toBe(true);
    expect(Object.isFrozen(grid.rows[0]?.cells[0])).toBe(true);
    expect(Object.isFrozen(grid.merges)).toBe(true);
  });

  it('gives the same grid for the bytes of the file', () => {
    const grid = viewer.describeSheet(MERGED).value;
    expect(viewer.describeSheetBytes(readFileSync(MERGED)).value).toEqual(grid);
  });

  it('takes the window and the sheet the request names', () => {
    const shown = (request: object) => viewer.describeSheet(MERGED, request).value?.shown;
    expect(shown({ range: 'A1:C3' })).toBe('A1:C3');
    expect(shown({ maxRows: 2 })).toBe('A1:E2');
    expect(shown({ maxColumns: 2 })).toBe('A1:B8');
    expect(shown({ sheet: 'Sheet1' })).toBe('A1:E8');
    expect(viewer.describeSheet(MERGED, { sheet: 0 }).value).toEqual(
      viewer.describeSheet(MERGED).value,
    );
    expect(viewer.describeSheet(MERGED, { sheet: null, range: null }).value?.shown).toBe('A1:E8');
    const bytes = readFileSync(MERGED);
    expect(viewer.describeSheetBytes(bytes, { sheet: 'Sheet1', maxRows: 2 }).value?.shown).toBe(
      'A1:E2',
    );
  });

  it('fails with sheet_not_found and a grid whose sheet is null', () => {
    const result = viewer.describeSheet(TABLE, { sheet: 'Nope' });
    expect([result.success, result.error?.code]).toEqual([false, 'sheet_not_found']);
    expect(result.value?.sheet).toBeNull();
    expect(result.value?.rows).toEqual([]);
  });

  it('fails with an io error and a grid for a file that does not exist', () => {
    const result = viewer.describeSheet('/nowhere/book.xlsx');
    expect([result.success, result.error?.code]).toEqual([false, 'io']);
    expect(result.value?.sheet).toBeNull();
  });

  it('fails with diagnostics and a grid for bytes that are not a workbook', () => {
    const result = viewer.describeSheetBytes(Buffer.from('not a zip'));
    expect(result.success).toBe(false);
    expect(result.value).not.toBeNull();
  });

  it('for a path with a lone surrogate, reads the file and shows the same grid', () => {
    const dir = mkdtempSync(join(tmpdir(), 'xlsx-graft-'));
    copyFileSync(MERGED, join(dir, '�.xlsx'));
    const request = { maxRows: 2 };
    expect(viewer.describeSheet(join(dir, '\uD800.xlsx'), request).value).toEqual(
      viewer.describeSheet(MERGED, request).value,
    );
    const missing = viewer.describeSheet(join(dir, '\uD800-missing.xlsx'));
    expect([missing.success, missing.value, missing.error?.code]).toEqual([false, null, 'io']);
  });

  it("throws a RangeError with the engine's message for a value the engine refuses", () => {
    for (const request of [{ maxRows: 0 }, { range: 'nope' }]) {
      expect(() => viewer.describeSheet(MERGED, request as never)).toThrow(RangeError);
    }
    expect(() => viewer.describeSheet(MERGED, { maxRows: 0 })).toThrow(/maxRows/);
    expect(() => viewer.describeSheet(MERGED, { sheet: 1.5 })).toThrow(RangeError);
    expect(() => viewer.describeSheet(MERGED, { maxColumns: Number.NaN })).toThrow(RangeError);
    expect(() => viewer.describeSheetBytes(readFileSync(MERGED), { maxRows: 0 })).toThrow(
      RangeError,
    );
  });

  it('throws a RangeError before it reads the path for a sheet or a range it cannot encode', () => {
    expect(() => viewer.describeSheet('/nowhere/book.xlsx', { sheet: '\uD800' })).toThrow(
      RangeError,
    );
    expect(() => viewer.describeSheetBytes(new Uint8Array(), { range: '\uD800' })).toThrow(
      RangeError,
    );
    // A path that is not well-formed UTF-16 and names no file: the options are
    // built before the path is read, so the request's RangeError comes, not an `io` failure.
    expect(() => viewer.describeSheet('/nowhere/\uD800.xlsx', { sheet: '\uD800' })).toThrow(
      RangeError,
    );
    const range = 'A'.repeat(MAX_OPTIONS_BYTES);
    expect(() => viewer.describeSheet(MERGED, { range })).toThrow(RangeError);
    expect(() => viewer.describeSheetBytes(new Uint8Array(), { range })).toThrow(RangeError);
  });

  it('throws a TypeError for a BigInt', () => {
    expect(() => viewer.describeSheet(MERGED, { maxRows: 1n as unknown as number })).toThrow(
      TypeError,
    );
  });
});

describe('new Viewer', () => {
  it('is frozen', () => {
    expect(Object.isFrozen(new Viewer())).toBe(true);
  });

  it('throws LibraryNotFoundError for an addon that lacks a view function', () => {
    vi.stubEnv('XLSX_GRAFT_ADDON', join(PACKAGE_ROOT, 'test', 'fixtures', 'version-only.cjs'));
    expect(() => new Viewer()).toThrow(LibraryNotFoundError);
    expect(() => new Viewer()).toThrow(/exports no `treePath`/);
  });

  it('fails every call with a panic error for an addon whose functions panic', () => {
    vi.stubEnv('XLSX_GRAFT_ADDON', join(PACKAGE_ROOT, 'test', 'fixtures', 'view-panic.cjs'));
    const viewer = new Viewer();
    const results = [
      viewer.tree(TABLE),
      viewer.treeBytes(new Uint8Array()),
      viewer.describeSheet(TABLE),
      viewer.describeSheetBytes(new Uint8Array()),
    ];
    for (const result of results) {
      expect([result.success, result.value, result.error?.code]).toEqual([false, null, 'panic']);
    }
  });

  it('throws an XlsxGraftError for an addon that refuses the call as invalid', () => {
    vi.stubEnv('XLSX_GRAFT_ADDON', join(PACKAGE_ROOT, 'test', 'fixtures', 'view-invalid-arg.cjs'));
    const viewer = new Viewer();
    expect(() => viewer.tree(TABLE)).toThrow(XlsxGraftError);
    expect(() => viewer.describeSheetBytes(new Uint8Array())).toThrow(XlsxGraftError);
  });
});
