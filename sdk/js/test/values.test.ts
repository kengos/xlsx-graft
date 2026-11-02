import { describe, expect, it } from 'vitest';
import { CalendarDate, CellError, TimeOfDay } from '../src/index.js';
import { Zone } from '../src/zone.js';

describe('the values', () => {
  it('print as ISO 8601 writes them, and an error value as its text', () => {
    expect(String(new CalendarDate(987, 1, 2))).toBe('0987-01-02');
    expect(String(new TimeOfDay(9, 5, 7, 42))).toBe('09:05:07.042');
    expect(`${new CellError('#DIV/0!')}`).toBe('#DIV/0!');
  });
});

describe('Zone', () => {
  it('places a wall-clock time at the offset the zone has then', () => {
    const tokyo = new Zone('Asia/Tokyo');
    expect(tokyo.place([2024, 1, 31, 9, 0, 0, 250])).toEqual(new Date('2024-01-31T00:00:00.250Z'));
    const newYork = new Zone('America/New_York');
    expect(newYork.place([2024, 7, 1, 12, 0, 0, 0])).toEqual(new Date('2024-07-01T16:00:00Z'));
    expect(newYork.place([1900, 3, 1, 0, 0, 0, 0])).toEqual(new Date('1900-03-01T05:00:00Z'));
  });

  it('says why a time the daylight saving skips or repeats has no one instant', () => {
    const newYork = new Zone('America/New_York');
    expect(newYork.place([2024, 3, 10, 2, 30, 0, 0])).toBe('skipped');
    expect(newYork.place([2024, 11, 3, 1, 30, 0, 0])).toBe('repeated');
  });

  it('takes a year below 100 as written', () => {
    expect(new Zone('UTC').place([99, 12, 31, 0, 0, 0, 0])).toEqual(
      new Date('0099-12-31T00:00:00Z'),
    );
    expect(new Zone('+09:00').place([99, 12, 31, 9, 0, 0, 0])).toEqual(
      new Date('0099-12-31T00:00:00Z'),
    );
  });
});
