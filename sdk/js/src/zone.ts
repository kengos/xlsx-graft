/**
 * Places a workbook's wall-clock date-time in the caller's time zone.
 *
 * A date-time cell has no zone, and a `Date` is an instant: the zone that
 * turns one into the other is the caller's to know, UTC unless told. Any
 * zone `Intl.DateTimeFormat` takes will do: an IANA name (`Asia/Tokyo`) or
 * an offset (`+09:00`).
 */

const DAY = 86_400_000;

/**
 * A wall-clock date-time, as the addon gives a date-time cell's `date` and
 * `time`: year, month from 1, day, hour, minute, second, millisecond.
 */
export type WallClock = readonly number[];

/**
 * Why a wall-clock time has no one instant in a zone: its daylight saving
 * skips it, or has it twice.
 */
export type Unplaced = 'skipped' | 'repeated';

export class Zone {
  readonly name: string;
  readonly #format: Intl.DateTimeFormat | null;

  /** Throws a RangeError for a zone `Intl.DateTimeFormat` does not take. */
  constructor(name: string) {
    let format: Intl.DateTimeFormat;
    try {
      format = new Intl.DateTimeFormat('en-US', {
        timeZone: name,
        hourCycle: 'h23',
        year: 'numeric',
        month: 'numeric',
        day: 'numeric',
        hour: 'numeric',
        minute: 'numeric',
        second: 'numeric',
      });
    } catch (error) {
      throw new RangeError(`timezone ${JSON.stringify(name)} is no zone Intl takes (${error})`);
    }
    this.name = name;
    // UTC, the default, has one offset: no need to ask Intl for it.
    this.#format = format.resolvedOptions().timeZone === 'UTC' ? null : format;
  }

  /**
   * The instant `wall` is in this zone, or why it has none. The offsets
   * looked at are the ones a day either side has, so a zone changing its
   * offset twice within two days is not told apart.
   */
  place(wall: WallClock): Date | Unplaced {
    const [year, month, day, hour, minute, second, ms] = wall as number[];
    const local = utc(year, month, day, hour, minute, second, ms);
    const offsets = new Set([this.#offset(local - DAY), this.#offset(local + DAY)]);
    const instants = [...offsets].map((offset) => local - offset);
    const found = instants.filter((instant) => this.#offset(instant) === local - instant);
    if (found.length === 1) {
      return new Date(found[0] as number);
    }
    return found.length === 0 ? 'skipped' : 'repeated';
  }

  /** How far ahead of UTC the wall clock is at `instant`, in ms. */
  #offset(instant: number): number {
    if (this.#format === null) {
      return 0;
    }
    const parts: Record<string, number> = {};
    for (const { type, value } of this.#format.formatToParts(instant)) {
      parts[type] = Number(value);
    }
    const { year, month, day, hour, minute, second } = parts;
    const wall = utc(year, month, day, hour, minute, second, 0);
    return wall - (instant - mod(instant, 1000));
  }
}

/** `wall` read as UTC, in ms; a year below 100 is not taken for 19xx. */
function utc(
  year: number,
  month: number,
  day: number,
  hour: number,
  minute: number,
  second: number,
  ms: number,
): number {
  const date = new Date(0);
  date.setUTCFullYear(year, month - 1, day);
  date.setUTCHours(hour, minute, second, ms);
  return date.getTime();
}

function mod(n: number, m: number): number {
  return ((n % m) + m) % m;
}
