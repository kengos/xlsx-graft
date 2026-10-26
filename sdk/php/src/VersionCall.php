<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Runs `xlsx-graft --version --format json` and returns the engine's version
 * as a {@see Result}.
 *
 * @internal
 */
final class VersionCall
{
    /**
     * The version string, such as `"0.0.0"`, on success.
     *
     * @return Result<string>
     *
     * @throws CliNotFoundException when the CLI cannot be run
     */
    public static function call(string $cli): Result
    {
        [$status, $signal, $stdout, $stderr] = CliProcess::run($cli, ['--version', '--format', 'json']);
        if ($status !== 0 || $signal !== null) {
            return Result::failure([CliDiagnostic::exitStatus($status, $signal, $stderr)]);
        }

        $decoded = json_decode($stdout);
        $version = $decoded instanceof \stdClass ? ($decoded->version ?? null) : null;
        if (!is_string($version)) {
            return Result::failure([CliDiagnostic::invalidOutput('a string "version"', $stdout)]);
        }

        return Result::ok($version);
    }
}
