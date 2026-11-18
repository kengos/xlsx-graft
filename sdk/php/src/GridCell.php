<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One cell of a grid that holds a value.
 */
final readonly class GridCell
{
    /**
     * @param string $ref the cell's reference (`B3`)
     * @param string $type the cell's type: `string`, `number`, `boolean`, `date`, `time`,
     *                     `datetime`, `elapsed` or `error`
     * @param string|int|float|bool $value the value of the cell's `value` key in the CLI's `--format json` output: a string (also for a date, a time,
     *                                     a date-time and an error), a bool, an int for a number
     *                                     written as an integer that fits in PHP's int (an elapsed
     *                                     time's milliseconds), or a float for any other number,
     *                                     as json_decode reads it
     * @param bool $truncated whether a long text was cut for display
     */
    public function __construct(
        public string $ref,
        public string $type,
        public string|int|float|bool $value,
        public bool $truncated,
    ) {}
}
