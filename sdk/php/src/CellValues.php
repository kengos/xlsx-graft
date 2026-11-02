<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Turns the cells of a `read` row line into the values of a {@see Row}, as
 * the docs of {@see Row} say.
 *
 * A date, time or date-time is the workbook's wall-clock time with no zone.
 * The date-time is placed in the caller's {@see Zone}: the zone is the
 * caller's to know, not the workbook's.
 *
 * @internal
 */
final class CellValues
{
    private const DATE = '(\d{4})-(\d{2})-(\d{2})';
    private const TIME = '(\d{2}):(\d{2}):(\d{2})\.(\d{3})';

    /**
     * The values of `$cells`; or why one of them cannot be given, as a
     * diagnostic: of a type this package does not know (a CLI newer than
     * the package, `unknown_value`), or a date-time the zone skips or has
     * twice (`datetime_unrepresentable`). Neither is guessed at.
     *
     * @param list<\stdClass> $cells
     *
     * @return list<string|float|bool|CalendarDate|TimeOfDay|\DateTimeImmutable|CellError|null>|Diagnostic
     *
     * @throws \UnexpectedValueException for a cell not of the contract's shape
     */
    public static function call(array $cells, Zone $zone): array|Diagnostic
    {
        $values = [];
        foreach ($cells as $cell) {
            $value = self::of(CliJson::string($cell, 'type'), CliJson::field($cell, 'value'), $zone);
            if ($value instanceof Diagnostic) {
                return $value;
            }
            $values[] = $value;
        }

        return $values;
    }

    private static function of(string $type, mixed $value, Zone $zone): string|float|bool|CalendarDate|TimeOfDay|\DateTimeImmutable|CellError|Diagnostic|null
    {
        return match ($type) {
            'empty' => $value === null ? null : self::refuse($type),
            'string' => is_string($value) ? $value : self::refuse($type),
            // A number always has a fraction or an exponent, but an integer
            // is the same number.
            'number' => is_float($value) || is_int($value) ? (float) $value : self::refuse($type),
            'boolean' => is_bool($value) ? $value : self::refuse($type),
            'date' => new CalendarDate(...self::parts(self::DATE, $type, $value)),
            'time' => new TimeOfDay(...self::parts(self::TIME, $type, $value)),
            'datetime' => self::moment(self::parts(self::DATE.'T'.self::TIME, $type, $value), $zone),
            'elapsed' => is_int($value) ? $value / 1000.0 : self::refuse($type),
            'error' => is_string($value) ? new CellError($value) : self::refuse($type),
            default => new Diagnostic('unknown_value', "the CLI returned a value of type {$type}, which this package does not know"),
        };
    }

    /**
     * The numbers of `$value`, a string `$pattern` matches whole.
     *
     * @return list<int>
     */
    private static function parts(string $pattern, string $type, mixed $value): array
    {
        is_string($value) && preg_match("/^{$pattern}$/D", $value, $parts) === 1 || self::refuse($type);

        return array_values(array_map(intval(...), array_slice($parts, 1)));
    }

    /** @param list<int> $wall */
    private static function moment(array $wall, Zone $zone): \DateTimeImmutable|Diagnostic
    {
        $placed = $zone->place(...$wall);
        if ($placed instanceof \DateTimeImmutable) {
            return $placed;
        }
        [$year, $month, $day, $hour, $minute, $second, $ms] = $wall;
        $shown = new CalendarDate($year, $month, $day).' '.new TimeOfDay($hour, $minute, $second, $ms);
        $why = $placed === 'skipped' ? 'its daylight saving skips it' : 'it has it twice';

        return new Diagnostic('datetime_unrepresentable', "{$shown} cannot be placed in {$zone->zone->getName()}: {$why}");
    }

    private static function refuse(string $type): never
    {
        throw new \UnexpectedValueException("a {$type} cell's value is not of its type");
    }
}
