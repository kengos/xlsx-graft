<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Reads what `xlsx-graft verify --format json` ended with as a {@see Result}
 * (docs/sdk.md § Verifying (`verify`)).
 *
 * @internal
 */
final class VerifyReport
{
    /**
     * A signal, or an exit status other than 0, 1 and 2, is `exit_status`. Exit
     * 0, 1 and 2 print the JSON object of the result: it is the result,
     * successful when its `ok` is true, its `error` only when `ok` is false.
     * Output that is no such object is `invalid_output` with exit 0 and
     * `exit_status` with exit 1 and 2 (the CLI ending with no result, or the
     * CLI's command-line parser refusing an argument this package built, with
     * nothing on stdout).
     *
     * @param int $status the exit status (-1 when killed by a signal)
     * @param int|null $signal the number of the signal that killed the CLI, null when it exited
     * @param string $stdout what the CLI printed to stdout
     * @param string $stderr what the CLI printed to stderr, quoted in an `exit_status` message
     *
     * @return Result<null>
     */
    public static function read(int $status, ?int $signal, string $stdout, string $stderr): Result
    {
        if ($signal !== null || $status < 0 || $status > 2) {
            return Result::failure([], CliDiagnostic::exitStatus($status, $signal, $stderr));
        }
        try {
            $object = CliJson::object($stdout);
            [$error, $records] = CliJson::report($object);
        } catch (\UnexpectedValueException) {
            return Result::failure([], $status === 0
                ? CliDiagnostic::invalidOutput('a boolean "ok"', trim($stdout))
                : CliDiagnostic::exitStatus($status, null, $stderr));
        }
        $diagnostics = array_column($records, 1);

        return CliJson::bool($object, 'ok') ? Result::ok(null, $diagnostics) : Result::failure($diagnostics, $error);
    }
}
