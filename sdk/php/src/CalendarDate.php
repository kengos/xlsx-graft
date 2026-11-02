<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * A date cell: a day with no time and no zone, as the workbook gives it.
 * PHP has no type for a date alone, and a `DateTimeImmutable` would put it
 * at a midnight in some zone the workbook never gave.
 */
final readonly class CalendarDate implements \Stringable
{
    public function __construct(
        public int $year,
        public int $month,
        public int $day,
    ) {}

    /** `2024-01-31`. */
    public function __toString(): string
    {
        return sprintf('%04d-%02d-%02d', $this->year, $this->month, $this->day);
    }
}
