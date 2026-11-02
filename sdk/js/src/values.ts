/**
 * What a read hands over: rows, their values and the sheets of a book.
 *
 * JavaScript has a type for an instant (`Date`) but none for a date alone or
 * a time of day on the lines this package supports, so those two are this
 * package's: a `Date` would put a date-only cell at a midnight, and a time of
 * day on a date, that the workbook never gave.
 */

/** A date with no time and no zone, as a date cell holds it. */
export class CalendarDate {
  readonly year: number;
  /** From 1, January. */
  readonly month: number;
  readonly day: number;

  constructor(year: number, month: number, day: number) {
    this.year = year;
    this.month = month;
    this.day = day;
    Object.freeze(this);
  }

  /** `2024-01-31`. */
  toString(): string {
    return `${pad(this.year, 4)}-${pad(this.month, 2)}-${pad(this.day, 2)}`;
  }
}

/** A time of day with no date and no zone, as a time cell holds it. */
export class TimeOfDay {
  readonly hour: number;
  readonly minute: number;
  readonly second: number;
  readonly millisecond: number;

  constructor(hour: number, minute: number, second: number, millisecond: number) {
    this.hour = hour;
    this.minute = minute;
    this.second = second;
    this.millisecond = millisecond;
    Object.freeze(this);
  }

  /** `18:00:00.000`. */
  toString(): string {
    const { hour, minute, second, millisecond } = this;
    return `${pad(hour, 2)}:${pad(minute, 2)}:${pad(second, 2)}.${pad(millisecond, 3)}`;
  }
}

/**
 * An error value, such as `#N/A`. Never a string: a cell holding the text
 * `#N/A` would be one too.
 */
export class CellError {
  readonly text: string;

  constructor(text: string) {
    this.text = text;
    Object.freeze(this);
  }

  toString(): string {
    return this.text;
  }
}

/**
 * One cell's value:
 *
 * | Cell     | Value                                                 |
 * | -------- | ----------------------------------------------------- |
 * | empty    | `null`                                                |
 * | string   | `string`                                              |
 * | number   | `number`                                              |
 * | boolean  | `boolean`                                             |
 * | date     | {@link CalendarDate}                                  |
 * | time     | {@link TimeOfDay}                                     |
 * | datetime | `Date`, the wall-clock time in the caller's zone      |
 * | elapsed  | `number` of milliseconds, not wrapped at a day        |
 * | error    | {@link CellError}                                     |
 */
export type Value = null | string | number | boolean | CalendarDate | TimeOfDay | Date | CellError;

/** One row a sheet writes: its number from 1, and its values from column A. */
export class Row {
  readonly number: number;
  readonly values: readonly Value[];

  constructor(number: number, values: readonly Value[]) {
    this.number = number;
    this.values = Object.freeze([...values]);
    Object.freeze(this);
  }
}

/** What a sheet holds: a `chartsheet` holds no cells to read. */
export type SheetKind = 'worksheet' | 'chartsheet' | 'dialogsheet' | 'macrosheet' | 'unknown';

/** Whether a sheet is shown. */
export type SheetState = 'visible' | 'hidden' | 'veryHidden' | 'unknown';

/** One sheet of a book, as the workbook lists it. */
export class SheetInfo {
  /** From 0, every sheet counted: what `book.sheet(index)` takes. */
  readonly index: number;
  readonly name: string;
  readonly kind: SheetKind;
  readonly state: SheetState;

  constructor(index: number, name: string, kind: SheetKind, state: SheetState) {
    this.index = index;
    this.name = name;
    this.kind = kind;
    this.state = state;
    Object.freeze(this);
  }
}

function pad(n: number, width: number): string {
  return String(n).padStart(width, '0');
}
