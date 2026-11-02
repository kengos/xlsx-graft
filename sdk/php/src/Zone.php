<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Places a workbook's wall-clock date-time in the caller's time zone.
 *
 * A date-time cell has no zone, and a `DateTimeImmutable` is an instant in
 * one: the zone is the caller's to know, UTC unless told. PHP places a wall
 * clock its daylight saving skips or has twice without a word (moving it on,
 * or taking the first), so each is checked against the zone's offsets before
 * it is placed.
 *
 * @internal
 */
final class Zone
{
    private const DAY = 86_400;

    /** Whether the zone has one offset (UTC, `+09:00`): nothing to check. */
    private bool $fixed;

    public function __construct(
        public readonly \DateTimeZone $zone,
    ) {
        $this->fixed = in_array($zone->getName()[0], ['+', '-'], true) || $zone->getName() === 'UTC';
    }

    /**
     * The instant the wall clock `$year`… `$millisecond` is in this zone,
     * or why it has none: `skipped` (its daylight saving skips it) or
     * `repeated` (it has it twice). The offsets looked at are those a day
     * either side has, so a zone changing its offset twice within two days
     * is not told apart.
     */
    public function place(int $year, int $month, int $day, int $hour, int $minute, int $second, int $millisecond): \DateTimeImmutable|string
    {
        $wall = sprintf('%04d-%02d-%02d %02d:%02d:%02d.%03d', $year, $month, $day, $hour, $minute, $second, $millisecond);
        if (!$this->fixed) {
            // gmmktime() would take a year below 101 for one of 1970 to 2069.
            $local = (new \DateTimeImmutable($wall, new \DateTimeZone('UTC')))->getTimestamp();
            $offsets = array_unique([$this->offset($local - self::DAY), $this->offset($local + self::DAY)]);
            $found = array_filter($offsets, fn(int $offset): bool => $this->offset($local - $offset) === $offset);
            if (count($found) !== 1) {
                return $found === [] ? 'skipped' : 'repeated';
            }
        }

        return new \DateTimeImmutable($wall, $this->zone);
    }

    /** How far ahead of UTC the wall clock is at `$instant`, in seconds. */
    private function offset(int $instant): int
    {
        return $this->zone->getOffset(new \DateTimeImmutable("@{$instant}"));
    }
}
