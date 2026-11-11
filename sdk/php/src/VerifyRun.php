<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One run of `xlsx-graft verify`: the files it needs (the workbook's bytes
 * and the policy, each in a file only this user may read), the arguments,
 * and the result, with the files removed after it however it ends.
 *
 * @internal
 */
final class VerifyRun
{
    /**
     * Verifies the workbook at `$path`.
     *
     * @param string|null $policy the policy as JSON, when there is one
     * @param list<string> $rules the arguments of the caller's rules (`--target=excel 2016`)
     * @param string $dir where the policy's file goes
     *
     * @return Result<null>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     */
    public static function path(string $cli, ?string $policy, string $path, array $rules, string $dir): Result
    {
        if (str_contains($path, "\0")) {
            return Result::failure([], new Diagnostic('invalid_arg', 'a path holds no NUL'));
        }

        return self::run($cli, $policy, $rules, $dir, $path, null);
    }

    /**
     * Verifies the workbook whose bytes `$data` holds, written to a file of
     * its own in `$dir`.
     *
     * @param string|null $policy the policy as JSON, when there is one
     * @param list<string> $rules the arguments of the caller's rules
     *
     * @return Result<null>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     */
    public static function bytes(string $cli, ?string $policy, string $data, array $rules, string $dir): Result
    {
        return self::run($cli, $policy, $rules, $dir, '', $data);
    }

    /**
     * @param list<string> $rules
     *
     * @return Result<null>
     */
    private static function run(string $cli, ?string $policy, array $rules, string $dir, string $path, ?string $data): Result
    {
        foreach ($rules as $rule) {
            if (str_contains($rule, "\0")) {
                return Result::failure([], new Diagnostic('invalid_arg', 'an argument holds no NUL'));
            }
        }
        $staged = [];
        try {
            if ($data !== null) {
                try {
                    $staged[] = $path = TemporaryFile::write($dir, '.xlsx', $data);
                } catch (\RuntimeException $e) {
                    return self::notWritten('the workbook', $e);
                }
            }
            $arguments = ['verify', '--format', 'json'];
            if ($policy !== null) {
                try {
                    $staged[] = $file = TemporaryFile::write($dir, '.json', $policy);
                } catch (\RuntimeException $e) {
                    return self::notWritten('the policy', $e);
                }
                $arguments[] = "--policy={$file}";
            }
            [$status, $signal, $stdout, $stderr] = CliProcess::run($cli, [...$arguments, ...$rules, '--', $path]);

            return VerifyReport::read($status, $signal, $stdout, $stderr);
        } finally {
            array_map(TemporaryFile::remove(...), $staged);
        }
    }

    /** @return Result<null> */
    private static function notWritten(string $what, \RuntimeException $why): Result
    {
        return Result::failure([], new Diagnostic('io', "{$what} could not be written to a temporary file ({$why->getMessage()})"));
    }
}
