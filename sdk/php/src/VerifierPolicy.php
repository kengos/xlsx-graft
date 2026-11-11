<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Writes the policy a {@see Verifier} is given as the JSON the CLI's
 * `--policy` file holds, whose keys are camelCase (docs/sdk.md § Verifying in
 * an SDK).
 *
 * @internal
 */
final class VerifierPolicy
{
    /** The deepest nesting of arrays {@see json_encode()} takes by default. */
    private const DEPTH = 512;

    /**
     * `$policy` as JSON, each string key written in camelCase (`max_entries`
     * as `maxEntries`, a key already in camelCase as it is), in the arrays
     * inside it too; the values and the integer keys stay as they are. An empty
     * `$policy` is `{}`, the empty policy, and an empty array inside it stays `[]`.
     *
     * @param array<mixed> $policy
     *
     * @throws \InvalidArgumentException when `$policy` cannot be written as JSON: a string that is not UTF-8, INF, NAN, or arrays nested more than 512 deep
     */
    public static function json(array $policy): string
    {
        try {
            return $policy === []
                ? '{}'
                : json_encode(self::camelized($policy, 1), JSON_THROW_ON_ERROR | JSON_PRESERVE_ZERO_FRACTION | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        } catch (\JsonException $e) {
            throw new \InvalidArgumentException("the policy cannot be written as JSON: {$e->getMessage()}", 0, $e);
        }
    }

    /**
     * @param array<mixed> $array
     *
     * @return array<mixed>
     *
     * @throws \JsonException when the arrays nest deeper than {@see json_encode()} writes
     */
    private static function camelized(array $array, int $depth): array
    {
        $depth > self::DEPTH && throw new \JsonException('Maximum stack depth exceeded');
        $camelized = [];
        foreach ($array as $key => $value) {
            $key = is_string($key) ? self::camel($key) : $key;
            $camelized[$key] = is_array($value) ? self::camelized($value, $depth + 1) : $value;
        }

        return $camelized;
    }

    private static function camel(string $key): string
    {
        return preg_replace_callback('/_([a-z\d])/', static fn(array $m): string => strtoupper($m[1]), $key) ?? $key;
    }
}
