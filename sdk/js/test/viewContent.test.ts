import { describe, expect, it } from 'vitest';
import * as viewContent from '../src/viewContent.js';

// Each JSON below holds a different value in every place of the same type
// (every string, every number), and its booleans are chosen so that any two
// keys differ somewhere, so a key read for another gives a different object.
// The expected values are written out in full and compared as a whole.

const PACKAGE_RELATIONSHIPS = [
  { id: 'rIdPackage1', type: 'urn:type:package-one', target: { part: '/xl/workbook.xml' } },
  {
    id: 'rIdPackage2',
    type: 'urn:type:package-two',
    target: { external: 'https://example.com/x' },
  },
];

const PART_ONE = {
  name: '/xl/workbook.xml',
  contentType: 'application/one',
  contentTypeKnown: true,
  size: 1101,
  compressedSize: 211,
  readable: false,
  referenced: true,
  relationshipsKnown: false,
  relationships: [
    { id: 'rIdPartOne', type: 'urn:type:part-one', target: { part: '/xl/sheets/a.xml' } },
  ],
};

// Where PART_ONE has `true`, a boolean here is `false`, and the other way round.
const PART_TWO = {
  name: '/xl/sheets/a.xml',
  contentType: null,
  contentTypeKnown: false,
  size: 1202,
  compressedSize: 312,
  readable: true,
  referenced: null,
  relationshipsKnown: true,
  relationships: [],
};

// With the two above, `readable` and `relationshipsKnown` differ only here.
const PART_THREE = {
  name: '/xl/sharedStrings.xml',
  contentType: 'application/three',
  contentTypeKnown: false,
  size: 1303,
  compressedSize: 413,
  readable: false,
  referenced: false,
  relationshipsKnown: true,
  relationships: [
    { id: 'rIdPartThree', type: 'urn:type:part-three', target: { external: 'file:///z' } },
  ],
};

const TREE = {
  complete: true,
  relationshipsKnown: false,
  relationships: PACKAGE_RELATIONSHIPS,
  parts: [PART_ONE, PART_TWO, PART_THREE],
};

// A second tree and grid with the other value of each boolean of the first.
const TREE_OTHER = {
  complete: false,
  relationshipsKnown: true,
  relationships: [],
  parts: [
    {
      name: '/docProps/app.xml',
      contentType: 'text/xml',
      contentTypeKnown: true,
      size: 1404,
      compressedSize: 514,
      readable: true,
      referenced: false,
      relationshipsKnown: false,
      relationships: [],
    },
  ],
};

const GRID = {
  sheet: { index: 3, name: 'Data', kind: 'chartsheet', state: 'hidden' },
  dimension: 'A1:E9',
  used: 'A1:D8',
  shown: 'A1:C3',
  truncated: { rows: true, columns: false, merges: false },
  complete: true,
  rows: [
    {
      row: 2,
      cells: [
        { ref: 'B2', type: 'string', value: 'text', truncated: true },
        { ref: 'C2', type: 'number', value: 4.5 },
      ],
    },
    { row: 7, cells: [{ ref: 'A7', type: 'boolean', value: true, truncated: false }] },
  ],
  merges: ['B2:C3', 'D4:E5'],
};

describe('viewContent.tree', () => {
  it('reads every key of the tree, its parts and their relationships', () => {
    expect(viewContent.tree({ ...TREE, extra: 1 })).toEqual({
      complete: true,
      relationshipsKnown: false,
      relationships: [
        {
          id: 'rIdPackage1',
          type: 'urn:type:package-one',
          target: { part: '/xl/workbook.xml', external: null },
        },
        {
          id: 'rIdPackage2',
          type: 'urn:type:package-two',
          target: { part: null, external: 'https://example.com/x' },
        },
      ],
      parts: [
        {
          name: '/xl/workbook.xml',
          contentType: 'application/one',
          contentTypeKnown: true,
          size: 1101,
          compressedSize: 211,
          readable: false,
          referenced: true,
          relationshipsKnown: false,
          relationships: [
            {
              id: 'rIdPartOne',
              type: 'urn:type:part-one',
              target: { part: '/xl/sheets/a.xml', external: null },
            },
          ],
        },
        {
          name: '/xl/sheets/a.xml',
          contentType: null,
          contentTypeKnown: false,
          size: 1202,
          compressedSize: 312,
          readable: true,
          referenced: null,
          relationshipsKnown: true,
          relationships: [],
        },
        {
          name: '/xl/sharedStrings.xml',
          contentType: 'application/three',
          contentTypeKnown: false,
          size: 1303,
          compressedSize: 413,
          readable: false,
          referenced: false,
          relationshipsKnown: true,
          relationships: [
            {
              id: 'rIdPartThree',
              type: 'urn:type:part-three',
              target: { part: null, external: 'file:///z' },
            },
          ],
        },
      ],
    });
  });

  it('reads a tree whose booleans are the other way round, with no relationships', () => {
    expect(viewContent.tree(TREE_OTHER)).toEqual({
      complete: false,
      relationshipsKnown: true,
      relationships: [],
      parts: [
        {
          name: '/docProps/app.xml',
          contentType: 'text/xml',
          contentTypeKnown: true,
          size: 1404,
          compressedSize: 514,
          readable: true,
          referenced: false,
          relationshipsKnown: false,
          relationships: [],
        },
      ],
    });
    expect(viewContent.tree({ ...TREE_OTHER, parts: [] }).parts).toEqual([]);
  });

  it('throws a TypeError for parts that are not an array, or a key that is missing', () => {
    expect(() => viewContent.tree({ ...TREE, parts: 1 })).toThrow(TypeError);
    expect(() => viewContent.tree({ ...TREE, parts: undefined })).toThrow(TypeError);
    const { complete: _, ...lacking } = TREE;
    expect(() => viewContent.tree(lacking)).toThrow(/no `complete`/);
  });
});

describe('viewContent.grid', () => {
  it('reads every key of the grid, its sheet, rows and cells', () => {
    expect(viewContent.grid({ ...GRID, extra: 1 })).toEqual({
      sheet: { index: 3, name: 'Data', kind: 'chartsheet', state: 'hidden' },
      dimension: 'A1:E9',
      used: 'A1:D8',
      shown: 'A1:C3',
      truncated: { rows: true, columns: false, merges: false },
      complete: true,
      rows: [
        {
          row: 2,
          cells: [
            { ref: 'B2', type: 'string', value: 'text', truncated: true },
            { ref: 'C2', type: 'number', value: 4.5, truncated: false },
          ],
        },
        { row: 7, cells: [{ ref: 'A7', type: 'boolean', value: true, truncated: false }] },
      ],
      merges: ['B2:C3', 'D4:E5'],
    });
  });

  it('reads a grid whose booleans are the other way round, with no cells', () => {
    const truncated = { rows: false, columns: false, merges: true };
    const sheet = { index: 0, name: 'Other', kind: 'worksheet', state: 'veryHidden' };
    const other = {
      ...GRID,
      sheet,
      dimension: null,
      used: null,
      shown: null,
      truncated,
      complete: false,
      rows: [],
      merges: [],
    };
    expect(viewContent.grid(other)).toEqual({
      sheet: { index: 0, name: 'Other', kind: 'worksheet', state: 'veryHidden' },
      dimension: null,
      used: null,
      shown: null,
      truncated: { rows: false, columns: false, merges: true },
      complete: false,
      rows: [],
      merges: [],
    });
  });

  it('reads a grid cut by the columns only', () => {
    const columns = { rows: false, columns: true, merges: false };
    expect(viewContent.grid({ ...GRID, truncated: columns }).truncated).toEqual(columns);
  });

  it('reads a sheet that is null as null', () => {
    expect(viewContent.grid({ ...GRID, sheet: null }).sheet).toBeNull();
  });

  it("reads a kind or a state this package does not know as 'unknown'", () => {
    const sheet = { index: 1, name: 'S', kind: 'futuresheet', state: 'sleeping' };
    expect(viewContent.grid({ ...GRID, sheet }).sheet).toEqual({
      index: 1,
      name: 'S',
      kind: 'unknown',
      state: 'unknown',
    });
  });

  it('throws a TypeError for merges or rows that are not an array', () => {
    expect(() => viewContent.grid({ ...GRID, merges: undefined })).toThrow(TypeError);
    expect(() => viewContent.grid({ ...GRID, merges: 1 })).toThrow(TypeError);
    expect(() => viewContent.grid({ ...GRID, rows: 1 })).toThrow(TypeError);
  });
});
