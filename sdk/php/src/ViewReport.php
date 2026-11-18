<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Reads what `xlsx-graft tree --format json` or `xlsx-graft describe-sheet
 * --format json` ended with as a {@see Result} (docs/sdk.md § Listing parts
 * (`tree`) and § Describing a sheet (`describe-sheet`)).
 *
 * @internal
 */
final class ViewReport
{
    /** What the exception says when the CLI refused an argument and wrote no reason. */
    public const NO_REASON = 'the CLI refused an argument (exit status 2) and gave no reason';

    /**
     * A signal, or an exit status other than 0, 1 and 2, is `exit_status`. So is
     * exit 2 of a command whose arguments the caller does not choose (`tree`):
     * only the CLI's command-line parser exits with it, so this package built a
     * command line wrong. Exit 2 of `describe-sheet` is a value the caller gave
     * that the CLI refuses: it throws an \InvalidArgumentException with the
     * reason the CLI wrote to stderr. Exit 0 and 1 print the JSON object of the
     * result: it is the result, successful when its `ok` is true. Its value is
     * set also when `ok` is false. Output that is no such object is
     * `invalid_output` with exit 0 and `exit_status` with exit 1.
     *
     * @template V of Tree|Grid
     *
     * @param int $status the exit status (-1 when killed by a signal)
     * @param int|null $signal the number of the signal that killed the CLI, null when it exited
     * @param string $stdout what the CLI printed to stdout
     * @param string $stderr what the CLI printed to stderr
     * @param bool $requestable whether exit 2 is a refused value of the caller
     * @param callable(\stdClass): V $content reads the object into the value
     * @param string $expected what the object must hold, quoted in an `invalid_output` message
     *
     * @return Result<V>
     *
     * @throws \InvalidArgumentException when `$requestable` and the CLI exited with 2
     */
    public static function read(int $status, ?int $signal, string $stdout, string $stderr, bool $requestable, callable $content, string $expected): Result
    {
        if ($signal === null && $status === 2 && $requestable) {
            // The CLI starts its own message with `xlsx-graft: `, which is not part of the reason.
            $reason = trim((string) preg_replace('/^xlsx-graft:(?:\s+|$)/', '', trim($stderr)));

            throw new \InvalidArgumentException($reason === '' ? self::NO_REASON : $reason);
        }
        if ($signal !== null || $status < 0 || $status > 1) {
            return Result::failure([], CliDiagnostic::exitStatus($status, $signal, $stderr));
        }
        try {
            $object = CliJson::object($stdout);
            [$error, $records] = CliJson::report($object);
            $ok = CliJson::bool($object, 'ok');
            $value = $content($object);
        } catch (\UnexpectedValueException) {
            return Result::failure([], $status === 0
                ? CliDiagnostic::invalidOutput($expected, trim($stdout))
                : CliDiagnostic::exitStatus($status, null, $stderr));
        }
        $diagnostics = array_column($records, 1);

        return $ok ? Result::ok($value, $diagnostics) : Result::failure($diagnostics, $error, $value);
    }
}
