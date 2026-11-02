/**
 * Turns the cells the addon's `rows.nextRow()` gives into {@link Value}s, as
 * the table on {@link Value} says.
 *
 * A date, time or date-time is the workbook's wall-clock time with no zone.
 * The date-time is placed in the caller's {@link Zone}: the zone is the
 * caller's to know, not the workbook's.
 */

import type { AddonCell } from './addon.js';
import { CalendarDate, CellError, TimeOfDay, type Value } from './values.js';
import type { WallClock, Zone } from './zone.js';

/**
 * A value that cannot be given: of a kind this package does not know (an
 * addon newer than the package, `unknown_value`), or a date-time the zone
 * skips or has twice (`datetime_unrepresentable`). Neither is guessed at.
 */
export class Refused {
  readonly code: string;
  readonly message: string;

  constructor(code: string, message: string) {
    this.code = code;
    this.message = message;
  }
}

/** The values of `cells`, or why one of them cannot be given. */
export function call(cells: readonly AddonCell[], zone: Zone): Value[] | Refused {
  const values: Value[] = [];
  for (const cell of cells) {
    const value = of(cell, zone);
    if (value instanceof Refused) {
      return value;
    }
    values.push(value);
  }
  return values;
}

// The addon sets the field a type names whenever it gives that type
// (engine/napi/src/call/rows.rs), so each is read without a check.
function of(cell: AddonCell, zone: Zone): Value | Refused {
  switch (cell.type) {
    case 'empty':
      return null;
    case 'string':
      return cell.text as string;
    case 'number':
      return cell.number as number;
    case 'boolean':
      return cell.boolean as boolean;
    case 'date': {
      const [year, month, day] = cell.date as number[];
      return new CalendarDate(year, month, day);
    }
    case 'time': {
      const [hour, minute, second, millisecond] = cell.time as number[];
      return new TimeOfDay(hour, minute, second, millisecond);
    }
    case 'datetime':
      return moment([...(cell.date as number[]), ...(cell.time as number[])], zone);
    case 'elapsed':
      return cell.milliseconds as number;
    case 'error':
      return new CellError(cell.text as string);
    default:
      return new Refused(
        'unknown_value',
        `the engine returned a value of type ${cell.type}, which this package does not know`,
      );
  }
}

function moment(wall: WallClock, zone: Zone): Date | Refused {
  const placed = zone.place(wall);
  if (placed instanceof Date) {
    return placed;
  }
  const [year, month, day, hour, minute, second, ms] = wall as number[];
  const shown = `${new CalendarDate(year, month, day)} ${new TimeOfDay(hour, minute, second, ms)}`;
  const why = placed === 'skipped' ? 'its daylight saving skips it' : 'it has it twice';
  return new Refused(
    'datetime_unrepresentable',
    `${shown} cannot be placed in ${JSON.stringify(zone.name)}: ${why}`,
  );
}
