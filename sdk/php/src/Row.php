<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One row a sheet writes: its number, from 1, and the value of each column
 * from A to its last cell, a column without one null.
 *
 * A value is null, a string, a float (`3.0` included), a bool, a
 * {@see CalendarDate}, a {@see TimeOfDay}, a `DateTimeImmutable` (a
 * date-time, in the zone the reader was given), a float of seconds (elapsed
 * time, not wrapped at a day) or a {@see CellError}.
 */
final readonly class Row
{
    /**
     * @param list<string|float|bool|CalendarDate|TimeOfDay|\DateTimeImmutable|CellError|null> $values
     */
    public function __construct(
        public int $number,
        public array $values,
    ) {}
}
