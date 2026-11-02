<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * A time cell: a time of day with no date and no zone, as the workbook gives
 * it. PHP has no type for one; a `DateTimeImmutable` would need a date the
 * workbook never gave, and seconds would be taken for elapsed time.
 */
final readonly class TimeOfDay implements \Stringable
{
    public function __construct(
        public int $hour,
        public int $minute,
        public int $second,
        public int $millisecond,
    ) {}

    /** `08:30:00.500`. */
    public function __toString(): string
    {
        return sprintf('%02d:%02d:%02d.%03d', $this->hour, $this->minute, $this->second, $this->millisecond);
    }
}
