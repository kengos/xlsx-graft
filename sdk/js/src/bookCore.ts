/**
 * The state of one open book that {@link Book} and {@link Sheet} share: the
 * addon's book, the rows open from it, and the failures the engine keeps
 * none of. Not exported from the package.
 *
 * JavaScript runs it on one thread, and nothing is raised between two of its
 * lines, so it takes no lock. The addon's objects own the C ABI's handles:
 * one this book drops is freed when the garbage collector finalizes it, on
 * that same thread.
 */

import type { AddonBook, AddonRows } from './addon.js';
import * as cellValues from './cellValues.js';
import type { Diagnostic } from './diagnostic.js';
import * as engineDiagnostics from './engineDiagnostics.js';
import { ClosedError } from './errors.js';
import { Result } from './result.js';
import * as statusDiagnostic from './statusDiagnostic.js';
import { Row, SheetInfo, type SheetKind, type SheetState } from './values.js';
import type { Zone } from './zone.js';

// What an addon newer than the package may name otherwise is `unknown`.
const KINDS: readonly SheetKind[] = ['worksheet', 'chartsheet', 'dialogsheet', 'macrosheet'];
const STATES: readonly SheetState[] = ['visible', 'hidden', 'veryHidden'];
const CLOSED = "the book is closed: use it inside XlsxReader.open's callback";

export class BookCore {
  readonly #book: AddonBook;
  readonly #zone: Zone;
  /** The rows open, each closed by its sheet's read or by the book's close. */
  readonly #rows = new Set<AddonRows>();
  /** Why this package stopped short where the engine keeps no failure. */
  #failure: Diagnostic | null = null;
  #report: engineDiagnostics.Report | null = null;
  #closed = false;

  constructor(book: AddonBook, zone: Zone) {
    this.#book = book;
    this.#zone = zone;
  }

  /** Throws a ClosedError once the book is closed. */
  check(): void {
    if (this.#closed) {
      throw new ClosedError(CLOSED);
    }
  }

  sheets(): SheetInfo[] {
    this.check();
    const { status, sheets } = this.#book.sheets();
    this.#ok(status);
    return sheets.map(
      ({ name, kind, state }, index) =>
        new SheetInfo(index, name, known(KINDS, kind), known(STATES, state)),
    );
  }

  /** The rows of the sheet `which` names, null when it is refused. */
  openRows(which: string | number): AddonRows | null {
    this.check();
    if (typeof which === 'string' && !which.isWellFormed()) {
      this.#ok(statusDiagnostic.INVALID_ARG);
      return null;
    }
    const rows = typeof which === 'string' ? this.#book.rowsNamed(which) : this.#book.rows(which);
    if (!this.#ok(rows.status)) {
      return null;
    }
    this.#rows.add(rows);
    return rows;
  }

  /**
   * The next row; null at the end of the sheet, or when reading it stopped
   * short, which the result says.
   */
  nextRow(rows: AddonRows): Row | null {
    this.check();
    const next = rows.nextRow();
    if (!this.#ok(next.status, next.message) || next.row === undefined) {
      return null;
    }
    const values = cellValues.call(next.row.cells, this.#zone);
    if (values instanceof cellValues.Refused) {
      this.#failWith(statusDiagnostic.of(values.code, `row ${next.row.number}: ${values.message}`));
      return null;
    }
    return new Row(next.row.number, values);
  }

  /** Closes `rows`, unless the book's close did. */
  closeRows(rows: AddonRows): void {
    if (this.#rows.delete(rows)) {
      rows.close();
    }
  }

  /**
   * Reads the diagnostics, then closes the book, which closes its rows, and
   * what is left of each rows. Once: a second close does nothing.
   */
  close(): void {
    if (this.#closed) {
      return;
    }
    this.#closed = true;
    try {
      this.#report = this.#readReport();
    } finally {
      this.#book.close();
      for (const rows of this.#rows) {
        rows.close();
      }
      this.#rows.clear();
    }
  }

  /**
   * `value` on success. Its error is the first failure; the engine's words
   * for it when the engine kept it too.
   */
  result<T>(value: T): Result<T> {
    const report = this.#report ?? engineDiagnostics.UNREAD;
    const error =
      this.#failure === null || this.#failure.code === report.error?.code
        ? report.error
        : this.#failure;
    if (error !== null || report.hasErrors) {
      return Result.failure(report.diagnostics, error);
    }
    return Result.ok(value, report.diagnostics);
  }

  /**
   * Whether `status` is OK. One that is not is kept as the failure, unless
   * one already is.
   */
  #ok(status: number, message?: string | null): boolean {
    if (status === statusDiagnostic.OK) {
      return true;
    }
    this.#failWith(statusDiagnostic.call(status, message));
    return false;
  }

  /** The first failure is the one kept. */
  #failWith(diagnostic: Diagnostic): void {
    this.#failure ??= diagnostic;
  }

  /** The book's diagnostics, null when they cannot be read. */
  #readReport(): engineDiagnostics.Report | null {
    const { status, value, message } = this.#book.diagnostics();
    if (!this.#ok(status, message)) {
      return null;
    }
    try {
      return engineDiagnostics.call(value as string);
    } catch (error) {
      this.#failWith(
        statusDiagnostic.of(
          'diagnostics_unreadable',
          `the engine's diagnostics could not be read: ${(error as Error).message}`,
        ),
      );
      return null;
    }
  }
}

function known<T extends string>(names: readonly T[], name: string): T | 'unknown' {
  return names.includes(name as T) ? (name as T) : 'unknown';
}
