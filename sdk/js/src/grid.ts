/** What {@link Viewer.describeSheet} shows: the cells of one sheet. */

import type { SheetInfo } from './values.js';

/** Which of a grid's limits cut what it shows. */
export class GridTruncated {
  /** Whether rows were left out. */
  readonly rows: boolean;
  /** Whether columns were left out. */
  readonly columns: boolean;
  /** Whether merged ranges were left out. */
  readonly merges: boolean;

  constructor(rows: boolean, columns: boolean, merges: boolean) {
    this.rows = rows;
    this.columns = columns;
    this.merges = merges;
    Object.freeze(this);
  }
}

/** One cell of a grid that holds a value. */
export class GridCell {
  /** The cell's reference (`B3`). */
  readonly ref: string;
  /**
   * The cell's type: `string`, `number`, `boolean`, `date`, `time`,
   * `datetime`, `elapsed` or `error`.
   */
  readonly type: string;
  /**
   * The JSON's value: a `string` (also for a date, a time, a date-time and an
   * error), a `boolean`, and a `number` for a number (an elapsed time's
   * milliseconds too). JSON does not tell `2.0` from `2`.
   */
  readonly value: string | number | boolean;
  /** Whether a long text was cut for display. */
  readonly truncated: boolean;

  constructor(ref: string, type: string, value: string | number | boolean, truncated: boolean) {
    this.ref = ref;
    this.type = type;
    this.value = value;
    this.truncated = truncated;
    Object.freeze(this);
  }
}

/** One row of a grid. */
export class GridRow {
  /** The row's number, from 1. */
  readonly row: number;
  /** The row's cells that hold a value, from the left. */
  readonly cells: readonly GridCell[];

  constructor(row: number, cells: readonly GridCell[]) {
    this.row = row;
    this.cells = Object.freeze([...cells]);
    Object.freeze(this);
  }
}

/**
 * The cells of one sheet, the value of `Viewer.describeSheet`. Names and text
 * are made for display (control characters removed, long text cut), so they
 * are not exact; read the rows with {@link XlsxReader} for exact values.
 */
export class Grid {
  /** The sheet shown, or `null` when none was found. */
  readonly sheet: SheetInfo | null;
  /** The range the sheet declares, or `null` when it declares none. */
  readonly dimension: string | null;
  /** The range that holds cells, or `null` when the sheet is empty. */
  readonly used: string | null;
  /** The range of the cells shown, or `null` when none are. */
  readonly shown: string | null;
  /** Which limits cut what is shown. */
  readonly truncated: GridTruncated;
  /** Whether the whole sheet was read. */
  readonly complete: boolean;
  /** The rows shown. */
  readonly rows: readonly GridRow[];
  /** The merged ranges (`A1:C1`). */
  readonly merges: readonly string[];

  constructor(
    sheet: SheetInfo | null,
    dimension: string | null,
    used: string | null,
    shown: string | null,
    truncated: GridTruncated,
    complete: boolean,
    rows: readonly GridRow[],
    merges: readonly string[],
  ) {
    this.sheet = sheet;
    this.dimension = dimension;
    this.used = used;
    this.shown = shown;
    this.truncated = truncated;
    this.complete = complete;
    this.rows = Object.freeze([...rows]);
    this.merges = Object.freeze([...merges]);
    Object.freeze(this);
  }
}
