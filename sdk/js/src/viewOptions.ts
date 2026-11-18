/**
 * Writes the options JSON of a view call, `{"sheet", "sheetIndex", "range",
 * "maxRows", "maxColumns"}`, from a {@link SheetRequest}. Not exported from the
 * package.
 */

import { MAX_OPTIONS_BYTES } from './verifier.js';
import type { SheetRequest } from './viewer.js';

/**
 * The options JSON of `request`: the keys that have a value, `sheet` as
 * `sheetIndex` when it is a number. The values are not checked: the engine
 * refuses a wrong one.
 *
 * Throws a RangeError for a `sheet` or a `range` that holds a lone surrogate
 * (written as U+FFFD it would name another sheet or range) and for options of
 * more than 2 MiB, and a TypeError for a value JSON cannot write.
 */
export function build(request: SheetRequest): string {
  const { sheet, range, maxRows, maxColumns } = request;
  for (const text of [sheet, range]) {
    if (typeof text === 'string' && !text.isWellFormed()) {
      throw new RangeError(
        'the sheet or the range holds a lone surrogate, which cannot be written as UTF-8',
      );
    }
  }
  const wanted = [
    typeof sheet === 'number' ? ['sheetIndex', sheet] : ['sheet', sheet],
    ['range', range],
    ['maxRows', maxRows],
    ['maxColumns', maxColumns],
  ] as const;
  const options = JSON.stringify(Object.fromEntries(wanted.filter(([, value]) => value != null)));
  const size = Buffer.byteLength(options);
  if (size > MAX_OPTIONS_BYTES) {
    throw new RangeError(
      `the options are ${size} bytes of JSON; the engine takes ${MAX_OPTIONS_BYTES}`,
    );
  }
  return options;
}
