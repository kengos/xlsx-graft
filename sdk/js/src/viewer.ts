/**
 * Shows a workbook's parts and a sheet's cells through the engine addon: the
 * engine's `tree` and `describe_sheet` views (docs/sdk.md § Views in an SDK).
 * The limits a workbook is read under are the CLI's defaults; no policy is
 * taken.
 */

import { type Addon, open as openAddon } from './addon.js';
import { LibraryNotFoundError } from './errors.js';
import type { Grid } from './grid.js';
import * as pathInput from './pathInput.js';
import type { Result } from './result.js';
import type { Tree } from './tree.js';
import * as viewContent from './viewContent.js';
import * as viewOptions from './viewOptions.js';
import * as viewReport from './viewReport.js';

export interface ViewerOptions {
  /** The engine addon's path, used when `XLSX_GRAFT_ADDON` is not set. */
  addon?: string;
}

/**
 * Which sheet of a workbook `describeSheet` shows and how much of it, each key
 * optional; `null` is the same as left out. A wrong value is not checked here:
 * the engine refuses it, and the call throws a RangeError with the engine's
 * message.
 */
export interface SheetRequest {
  /** The sheet's name, or its index from 0 when a number; the first sheet when left out. */
  sheet?: string | number | null;
  /** The range to show, in A1 form (`"A1:H50"`): the cells shown start at its top left cell and never go past it. Left out, the cells shown start at A1 and run to `maxRows` rows and `maxColumns` columns. */
  range?: string | null;
  /** The most rows to show, 1 to 10000; 100 when left out. */
  maxRows?: number | null;
  /** The most columns to show, 1 to 1000; 30 when left out. */
  maxColumns?: number | null;
}

/**
 * Shows the parts of a workbook and the cells of its sheets: the engine's
 * `tree` and `describe_sheet` views.
 *
 * A viewer does not change once it is made, and a call keeps no state in it.
 * A JavaScript object is not shared between worker threads, so a worker makes
 * its own viewer.
 *
 * @example
 * ```ts
 * const viewer = new Viewer();
 * for (const part of viewer.tree('book.xlsx').value?.parts ?? []) {
 *   console.log(part.name, part.contentType);
 * }
 *
 * const result = viewer.describeSheet('book.xlsx', { sheet: 'Data', range: 'A1:H50', maxRows: 20 });
 * for (const row of result.value?.rows ?? []) {
 *   console.log(row.cells.map((cell) => cell.value).join(', '));
 * }
 * ```
 */
export class Viewer {
  readonly #addon: Addon;

  /**
   * Throws a {@link LibraryNotFoundError} when no engine addon is found or the
   * addon of an older engine lacks the view functions.
   */
  constructor({ addon }: ViewerOptions = {}) {
    const loaded = openAddon({ path: addon }).addon;
    if (
      typeof loaded.treePath !== 'function' ||
      typeof loaded.treeBytes !== 'function' ||
      typeof loaded.describeSheetPath !== 'function' ||
      typeof loaded.describeSheetBytes !== 'function'
    ) {
      throw new LibraryNotFoundError(
        'the engine addon exports no `treePath`, `treeBytes`, `describeSheetPath` or `describeSheetBytes`',
      );
    }
    this.#addon = loaded;
    Object.freeze(this);
  }

  /**
   * The parts of the workbook at `path`, a path or a `file:` URL, and the
   * relationships between them.
   *
   * The result is successful when the engine's `ok` is true. Its `value` is a
   * {@link Tree} also when it is not (a workbook that is broken, or a file
   * that cannot be opened, gives a tree with the parts that were read, perhaps
   * none); it is `null` only when this package failed to get one (`panic`,
   * `null_arg`, `unknown_status`, `diagnostics_unreadable`, or `io` for a path
   * that is not well-formed UTF-16 whose file this package cannot read). A
   * file that cannot be opened is an `io` `error`. For a path with a lone
   * surrogate the engine cannot be given the name, so this package reads the
   * file and shows its bytes.
   *
   * Throws a TypeError for a URL that is not a `file:` URL.
   */
  tree(path: string | URL): Result<Tree> {
    return pathInput.call(
      path,
      (name) => viewReport.call(this.#addon.treePath(name, ''), viewContent.tree),
      (data) => viewReport.call(this.#addon.treeBytes(data, ''), viewContent.tree),
    );
  }

  /**
   * As {@link Viewer.tree}, the workbook being the bytes of `data`, read in
   * place for the call and not copied: JavaScript waits on the call, so `data`
   * does not change during it. A `SharedArrayBuffer` another thread writes
   * during the call is the caller's to prevent.
   */
  treeBytes(data: Uint8Array): Result<Tree> {
    return viewReport.call(this.#addon.treeBytes(view(data), ''), viewContent.tree);
  }

  /**
   * The cells of one sheet of the workbook at `path`, a path or a `file:` URL;
   * the first sheet when `request` names none.
   *
   * The result is successful when the engine's `ok` is true. Its `value` is a
   * {@link Grid} also when it is not: a sheet that is not found
   * (`sheet_not_found`) gives a grid whose `sheet` is `null`. It is `null`
   * only when this package failed to get one, as for {@link Viewer.tree}.
   *
   * Throws a RangeError for a value the engine refuses (`range: 'nope'`,
   * `maxRows: 0`, `sheet: 1.5`), with the engine's message, and for a `sheet`
   * or a `range` with a lone surrogate or options of more than 2 MiB, before
   * the engine is called; a TypeError for a value JSON cannot write (a BigInt)
   * or a URL that is not a `file:` URL.
   */
  describeSheet(path: string | URL, request: SheetRequest = {}): Result<Grid> {
    const options = viewOptions.build(request);
    return pathInput.call(
      path,
      (name) => viewReport.call(this.#addon.describeSheetPath(name, options), viewContent.grid),
      (data) => viewReport.call(this.#addon.describeSheetBytes(data, options), viewContent.grid),
    );
  }

  /**
   * As {@link Viewer.describeSheet}, the workbook being the bytes of `data`,
   * read in place as {@link Viewer.treeBytes} reads it.
   */
  describeSheetBytes(data: Uint8Array, request: SheetRequest = {}): Result<Grid> {
    const options = viewOptions.build(request);
    return viewReport.call(this.#addon.describeSheetBytes(view(data), options), viewContent.grid);
  }
}

// A view of the same memory, as the addon takes a Buffer.
function view(data: Uint8Array): Buffer {
  return Buffer.from(data.buffer, data.byteOffset, data.byteLength);
}
