/** A workbook open to read, and its sheets. */

import type { BookCore } from './bookCore.js';
import type { Row, SheetInfo } from './values.js';

/** The largest index the addon takes (a u32). */
const MAX_INDEX = 0xffff_ffff;

/**
 * A workbook open to read, handed to the callback of `XlsxReader.open`: its
 * sheets, and the rows of each through {@link Book.sheet}.
 *
 * Usable inside that callback only. When it returns or throws, the book is
 * closed, and with it every sheet's rows; using it then throws a
 * `ClosedError`.
 *
 * Nothing here throws for what the workbook holds. A sheet the engine
 * refuses (no such sheet, not a worksheet) reads no rows, and the `Result`
 * fails saying why, as it does when a problem stops a sheet's rows or the
 * host fails. A panic in the engine ends every sheet of the book: none reads
 * a row after it, and the book lists no sheets.
 */
export class Book {
  readonly #core: BookCore;

  /** @internal `XlsxReader.open` makes the book. */
  constructor(core: BookCore) {
    this.#core = core;
  }

  /**
   * The sheets, in the order the workbook lists them, hidden ones included;
   * none when the workbook cannot be read.
   */
  sheets(): SheetInfo[] {
    return this.#core.sheets();
  }

  /**
   * The sheet named `which` (letter case aside) or at index `which` (from 0,
   * hidden sheets and chartsheets counted); the first when not given.
   * Nothing is read until {@link Sheet.rows}. A name or an index no sheet has
   * is the engine's to refuse there; to look for a sheet, look in
   * {@link Book.sheets}: a refused sheet fails the `Result`.
   *
   * Throws a TypeError for neither a string nor a number, and a RangeError
   * for a number that is not an integer from 0 to 2^32 - 1.
   */
  sheet(which: string | number = 0): Sheet {
    this.#core.check();
    if (typeof which === 'number') {
      if (!Number.isInteger(which) || which < 0 || which > MAX_INDEX) {
        throw new RangeError(`a sheet index is an integer from 0 to ${MAX_INDEX}, not ${which}`);
      }
    } else if (typeof which !== 'string') {
      throw new TypeError(`a sheet is a name or an index from 0, not ${String(which)}`);
    }
    return new Sheet(this.#core, which);
  }
}

/** One sheet of a {@link Book}, asked for by name or index. */
export class Sheet {
  readonly #core: BookCore;
  readonly #which: string | number;

  /** @internal {@link Book.sheet} makes it. */
  constructor(core: BookCore, which: string | number) {
    this.#core = core;
    this.#which = which;
  }

  /**
   * Each row the sheet writes, in order, read one at a time as the iterator
   * is: no more of the sheet is held than the row given. Each call reads the
   * sheet from its start, opening it at the first row asked for.
   *
   * Reading stops at the first problem in the sheet, the rows before it
   * having come, and gives nothing when the engine refuses the sheet; either
   * way the `Result` fails, saying why. An iterator neither read to its end
   * nor closed (a `for...of` that breaks closes it) holds its rows until the
   * book's callback returns.
   *
   * Throws a `ClosedError` at the first row asked for after the book's
   * callback returned.
   */
  *rows(): Generator<Row, void, undefined> {
    const rows = this.#core.openRows(this.#which);
    if (rows === null) {
      return;
    }
    try {
      for (let row = this.#core.nextRow(rows); row !== null; row = this.#core.nextRow(rows)) {
        yield row;
      }
    } finally {
      this.#core.closeRows(rows);
    }
  }
}
