<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Reads the JSON `xlsx-graft sheets`, `xlsx-graft read` and
 * `xlsx-graft verify` print, a line at a time, holding each line to its
 * contract (docs/sdk.md § Listing sheets, § Reading rows and § Verifying
 * (`verify`)): a key missing or of another type throws an
 * \UnexpectedValueException, which the caller turns into `invalid_output`.
 *
 * @internal
 */
final class CliJson
{
    /** The JSON object `$line` is. */
    public static function object(string $line): \stdClass
    {
        try {
            $value = json_decode($line, false, 512, JSON_THROW_ON_ERROR);
        } catch (\JsonException $e) {
            throw new \UnexpectedValueException("not JSON: {$e->getMessage()}");
        }

        return self::asObject($value);
    }

    /** `$object`'s `$key`, which it must have. */
    public static function field(\stdClass $object, string $key): mixed
    {
        property_exists($object, $key) || throw new \UnexpectedValueException("no \"{$key}\"");

        return $object->{$key};
    }

    public static function string(\stdClass $object, string $key): string
    {
        $value = self::field($object, $key);

        return is_string($value) ? $value : throw new \UnexpectedValueException("\"{$key}\" is not a string");
    }

    public static function int(\stdClass $object, string $key): int
    {
        $value = self::field($object, $key);

        return is_int($value) ? $value : throw new \UnexpectedValueException("\"{$key}\" is not an integer");
    }

    public static function bool(\stdClass $object, string $key): bool
    {
        $value = self::field($object, $key);

        return is_bool($value) ? $value : throw new \UnexpectedValueException("\"{$key}\" is not a boolean");
    }

    /** @return list<\stdClass> */
    public static function objects(\stdClass $object, string $key): array
    {
        $value = self::field($object, $key);
        is_array($value) && array_is_list($value) || throw new \UnexpectedValueException("\"{$key}\" is not an array");

        return array_map(self::asObject(...), $value);
    }

    /**
     * The `error` and the `diagnostics` of the object `sheets` or `verify` prints,
     * or of `read`'s end line: why the engine refused what was asked, and every
     * problem it found, in order, each with the JSON it was read from, so
     * the same one read by two runs can be told.
     *
     * @return array{Diagnostic|null, list<array{string, Diagnostic}>}
     */
    public static function report(\stdClass $object): array
    {
        self::bool($object, 'ok');
        $error = self::field($object, 'error');
        if ($error !== null) {
            $error = self::asObject($error);
            $error = new Diagnostic(self::string($error, 'code'), self::string($error, 'message'));
        }
        $diagnostics = [];
        foreach (self::objects($object, 'diagnostics') as $record) {
            $diagnostics[] = [(string) json_encode($record), new Diagnostic(
                self::string($record, 'code'),
                self::string($record, 'message'),
                self::string($record, 'severity'),
                self::map(self::field($record, 'args')),
                self::map(self::field($record, 'location')),
                self::nullableMap(self::field($record, 'decidedBy')),
                self::nullableMap(self::field($record, 'fix')),
            )];
        }

        return [$error, $diagnostics];
    }

    private static function asObject(mixed $value): \stdClass
    {
        return $value instanceof \stdClass ? $value : throw new \UnexpectedValueException('not a JSON object: '.json_encode($value));
    }

    /**
     * A JSON object as an array, the objects in it too.
     *
     * @return array<mixed>
     */
    private static function map(mixed $value): array
    {
        $map = json_decode((string) json_encode(self::asObject($value)), true);

        return is_array($map) ? $map : [];
    }

    /** @return array<mixed>|null */
    private static function nullableMap(mixed $value): ?array
    {
        return $value === null ? null : self::map($value);
    }
}
